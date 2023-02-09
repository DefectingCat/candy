use anyhow::Result;
use log::error;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

type Reader<'a> = &'a mut BufReader<&'a mut &'a TcpStream>;

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
pub fn read_request(reader: Reader) -> Result<String> {
    let mut request_string = String::new();
    loop {
        let byte = reader.read_line(&mut request_string)?;
        if byte < 3 {
            break;
        }
    }
    Ok(request_string)
}

/// Read request body with Content-Length.
pub fn read_body(reader: Reader, size: usize) -> Result<String> {
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

pub fn handle_get() {}

pub fn handle_post(reader: Reader, headers: &HashMap<String, String>) {
    let size = headers
        .get("Content-Length")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let body = read_body(reader, size);
}

pub fn handle_error() {}

pub fn handle_connection(mut stream: &TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    // Read http request bytes to string.
    let request_string = match read_request(&mut buf_reader) {
        Ok(res) => res,
        Err(err) => {
            error!("failed to parse request {}", err.to_string());
            return handle_error();
        }
    };
    // Read string to lines.
    let request: Vec<_> = request_string.lines().collect();
    // HTTP method in first line.
    let first_line = match request.first() {
        Some(res) => *res,
        None => {
            error!("failed to parse request method");
            return handle_error();
        }
    };

    let mut router = HashMap::new();
    let first_line: Vec<_> = first_line.split(' ').collect();
    router.insert("method", first_line.first());
    router.insert("route", first_line.get(1));
    // Parse request headers.
    let headers = collect_headers(&request);

    let method = if let Some(Some(m)) = router.get("method") {
        **m
    } else {
        return handle_error();
    };
    match method {
        "GET" => {}
        "POST" => handle_post(&mut buf_reader, &headers),
        _ => return handle_error(),
    }

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("./static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
