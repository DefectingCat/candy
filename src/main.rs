use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ptr::hash;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4000").expect("cannon listen on port 4000");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(&stream);
    }
}

fn handle_connection(mut stream: &TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let msg: Vec<_> = buf_reader.bytes().map(|byte| byte.unwrap()).collect();
    let request: Vec<_> = msg
        .lines()
        .map(|line| line.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    let mut headers = HashMap::new();
    request.iter().for_each(|header| {
        if let Some(head) = header.split_once(": ") {
            headers.entry(head.0).or_insert(head.1);
        }
    });
    dbg!(&headers);
    let str = String::from_utf8_lossy(&msg);
    // buf_reader.lines().for_each(|result| {
    //     let result = result.unwrap();
    //     dbg!(&result);
    // });
    // @TODO Use Content-Length to read post body.
    // let http_request: Vec<_> = buf_reader
    //     .lines()
    //     .map(|result| result.unwrap())
    //     .take_while(|line| !line.is_empty())
    //     .collect();
    // let first_line = &http_request[0];
    //
    // dbg!(&http_request);

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("./static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
