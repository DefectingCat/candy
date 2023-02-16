use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::{debug, error, info};

use crate::config::Config;

// type Reader<'a> = &'a mut BufReader<&'a TcpStream>;

/// Collect request string with Hashmap to headers.
pub fn collect_headers(request: &[&str]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    request.iter().for_each(|header| {
        if let Some(head) = header.split_once(": ") {
            headers
                .entry(head.0.to_string())
                .or_insert(head.1.to_string());
        }
    });
    headers
}

/// Read http request to string.
pub fn read_request(reader: &mut BufReader<&mut &TcpStream>) -> Result<String> {
    let mut request_string = String::new();
    loop {
        let byte = reader.read_line(&mut request_string)?;
        if byte < 3 {
            break;
        }
    }
    Ok(request_string)
}

/// Read rest request body with Content-Length.
pub fn read_body(reader: &mut BufReader<&mut &TcpStream>, size: usize) -> Result<String> {
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Handle get request.
/// @params path: static file folder path in config.
/// @params route: request route.
pub fn handle_get(path: &PathBuf, route: &str) -> Result<String> {
    let status_line = "HTTP/1.1 200 OK";
    let mut path = PathBuf::from(path);
    path.push(route.replace('/', ""));
    path.push("index.html");
    debug!("{path:?}");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Can not read file {path:?}"))?;
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");
    Ok(response)
}

pub fn handle_post(
    reader: &mut BufReader<&mut &TcpStream>,
    headers: &HashMap<String, String>,
) -> Result<String> {
    let size = headers
        .get("Content-Length")
        .expect("cannot read Content-Length")
        .parse::<usize>()?;
    let body = read_body(reader, size);
    debug!("{body:?}");

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("./static/index.html")?;
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");
    Ok(response)
}

pub fn handle_error(mut stream: &TcpStream) {
    let status_line = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
    let response = status_line.to_string();
    stream.write_all(response.as_bytes()).unwrap();
}

pub fn handle_not_found(path: &PathBuf, mut stream: &TcpStream) {
    let status_line = "HTTP/1.1 404 Not Found";

    let mut path = PathBuf::from(path);
    path.push("404.html");
    let contents =
        match fs::read_to_string(&path).with_context(|| format!("Can not read file {path:?}")) {
            Ok(c) => c,
            Err(err) => {
                error!("{}", err.to_string());
                return handle_error(stream);
            }
        };
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

pub fn handle_connection(mut stream: &TcpStream, config: Arc<Mutex<Config>>) {
    let mut buf_reader = BufReader::new(&mut stream);
    // Read http request bytes to string.
    let request_string = match read_request(&mut buf_reader) {
        Ok(res) => res,
        Err(err) => {
            error!("failed to parse request {}", err.to_string());
            return handle_error(stream);
        }
    };
    // Read string to lines.
    let request: Vec<_> = request_string.lines().collect();
    // HTTP method in first line.
    let first_line = match request.first() {
        Some(res) => *res,
        None => {
            error!("failed to parse request method");
            return handle_error(stream);
        }
    };
    let headers = collect_headers(&request);

    // Print request log.
    let mut log_info = format!("\"{first_line}\"");
    ["Host", "User-Agent"].iter().for_each(|name| {
        if let Some(info) = headers.get(*name) {
            log_info.push_str(&format!(" - \"{info}\""))
        }
    });
    info!("{log_info}");

    let mut router = HashMap::new();
    let first_line: Vec<_> = first_line.split(' ').collect();
    router.insert("method", first_line.first());
    router.insert("route", first_line.get(1));

    // Parse request headers.
    let method = if let Some(Some(m)) = router.get("method") {
        **m
    } else {
        return handle_error(stream);
    };
    let route = if let Some(Some(r)) = router.get("route") {
        **r
    } else {
        return handle_error(stream);
    };
    let response = match method {
        "GET" => {
            let config = config.lock();
            // let path = &config.host.root_folder;
            let path = match &config {
                Ok(config) => &config.host.root_folder,
                Err(err) => {
                    error!("failed lock config {}", err.to_string());
                    return handle_error(stream);
                }
            };
            match handle_get(path, route) {
                Ok(res) => res,
                Err(err) => {
                    return match err.downcast_ref::<Error>() {
                        Some(error) => {
                            if let ErrorKind::NotFound = error.kind() {
                                return handle_not_found(path, stream);
                            }
                        }
                        _ => {
                            error!("Failed to handle get: {}", err.to_string());
                            handle_error(stream)
                        }
                    }
                }
            }
        }
        "POST" => {
            if let Ok(res) = handle_post(&mut buf_reader, &headers) {
                res
            } else {
                return handle_error(stream);
            }
        }
        _ => return handle_error(stream),
    };

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
