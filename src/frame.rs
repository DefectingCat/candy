use anyhow::Result;
use log::error;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{BufRead, BufReader},
    net::TcpStream,
};

#[derive(Debug)]
pub struct FrameError {
    details: String,
}

impl FrameError {
    fn new(msg: &str) -> Self {
        Self {
            details: msg.to_string(),
        }
    }
}

impl Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for FrameError {
    fn description(&self) -> &str {
        return &self.details;
    }
}

pub struct HttpFrame<'a> {
    pub request_str: String,
    pub headers: HashMap<String, String>,
    pub method: &'a str,
    pub path: &'a str,
}

impl<'a> HttpFrame<'a> {
    fn new(reader: &mut BufReader<&mut &TcpStream>) -> Result<Self, FrameError> {
        let mut request_str = match read_request(reader) {
            Ok(str) => str,
            Err(err) => {
                error!("{}", err.description());
                Err(FrameError(&err.description()));
            }
        };

        // Read string to lines.
        let request: Vec<_> = request_str.lines().collect();
        // HTTP method in first line.
        let first_line = match request.first() {
            Some(res) => *res,
            None => {
                error!("failed to parse request method");
                return Err(FrameError::new("failed to parse request method"));
            }
        };

        Ok(Self {
            request_str,
            headers: (),
            method: (),
            path: (),
        })
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
