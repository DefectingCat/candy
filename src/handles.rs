use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{debug, error, info};

use crate::config::Config;
use crate::consts::NOT_FOUND;
use crate::error::CandyError;
use crate::frame::HttpFrame;

// type Reader<'a> = &'a mut BufReader<&'a TcpStream>;

/// Read rest request body with Content-Length.
pub fn read_body(reader: &mut BufReader<&mut &TcpStream>, size: usize) -> Result<String> {
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Handle get request.
/// @params path: static file folder path in config.
/// @params route: request route.
pub fn handle_get(path: &PathBuf, route: &str) -> Result<String, CandyError> {
    let status_line = "HTTP/1.1 200 OK";
    let mut path = PathBuf::from(path);
    path.push(route.replace('/', ""));
    path.push("index.html");
    debug!("{path:?}");

    let contents = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            debug!("{err:?}");
            match err.kind() {
                ErrorKind::NotFound => {
                    return Err(CandyError::NotFound(err));
                }
                _ => return Err(CandyError::Unknown),
            }
        }
    };
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

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => NOT_FOUND.to_string(),
            _ => {
                debug!("{err:?}");
                return handle_error(stream);
            }
        },
    };
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

pub fn handle_connection(mut stream: &TcpStream, config: Arc<Mutex<Config>>) {
    let mut buf_reader = BufReader::new(&mut stream);

    let HttpFrame {
        request_str,
        headers,
        router,
    } = match HttpFrame::new(&mut buf_reader) {
        Ok(frame) => frame,
        Err(err) => {
            error!("{:?}", err.to_string());
            return handle_error(stream);
        }
    };

    if let Some(line) = request_str.lines().next() {
        // Print request log.
        let mut log_info = format!("\"{line}\"");
        ["Host", "User-Agent"].iter().for_each(|name| {
            if let Some(info) = headers.get(*name) {
                log_info.push_str(&format!(" - \"{info}\""))
            }
        });
        info!("{log_info}");
    }

    // Parse request headers.
    let method = if let Some(Some(m)) = router.get("method") {
        &m[..]
    } else {
        return handle_error(stream);
    };
    let route = if let Some(Some(r)) = router.get("path") {
        &r[..]
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
                    return match err {
                        CandyError::NotFound(_) => {
                            return handle_not_found(path, stream);
                        }
                        _ => {
                            error!("Failed to handle get: {}", err.to_string());
                            handle_error(stream)
                        }
                    };
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
