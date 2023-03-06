use anyhow::Result;
use log::error;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    net::TcpStream,
};

pub struct HttpFrame<'a> {
    pub request_str: String,
    pub headers: HashMap<String, String>,
    pub method: &'a str,
    pub path: &'a str,
}

impl<'a> HttpFrame<'a> {
    fn new(reader: &mut BufReader<&mut &TcpStream>) -> Result<Self> {
        let mut request_str = read_request(reader)?;

        // Read string to lines.
        let request: Vec<_> = request_str.lines().collect();
        // HTTP method in first line.
        let first_line = match request.first() {
            Some(res) => *res,
            None => {
                error!("failed to parse request method");
                return handle_error(stream);
            }
        };

        Self {
            request_str,
            headers: (),
            method: (),
            path: (),
        }
    }
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
