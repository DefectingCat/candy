use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

mod config;
mod thread_pool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4000").expect("cannon listen on port 4000");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(&stream);
    }
}

/// Collect request string with Hashmap to headers.
fn collect_headers(request: &[&str]) -> HashMap<String, String> {
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
fn read_request(reader: &mut BufReader<&mut &TcpStream>) -> String {
    let mut request_string = String::new();
    loop {
        let byte = reader.read_line(&mut request_string).unwrap();
        if byte < 3 {
            break;
        }
    }
    request_string
}

/// Read request body with Content-Length.
fn read_body(reader: &mut BufReader<&mut &TcpStream>, size: usize) -> String {
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer).unwrap();
    String::from_utf8_lossy(&buffer).to_string()
}

fn handle_connection(mut stream: &TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    // Read http request bytes to string.
    let request_string = read_request(&mut buf_reader);
    // Read string to lines.
    let request: Vec<_> = request_string.lines().collect();
    // HTTP method in first line.
    let request_method = request.first().unwrap();
    // Parse request headers.
    let headers = collect_headers(&request);
    let size = headers
        .get("Content-Length")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let body = read_body(&mut buf_reader, size);

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("./static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
