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

pub struct HttpFrame {
    request_str: String,
    pub headers: HashMap<String, String>,
    pub first_line: String,
    pub router: HashMap<&'static str, Option<String>>,
}

impl HttpFrame {
    fn new(reader: &mut BufReader<&mut &TcpStream>) -> Result<Self, FrameError> {
        let request_str = match read_request(reader) {
            Ok(str) => str,
            Err(err) => {
                error!("{:?}", err);
                return Err(FrameError::new(&err.to_string()));
            }
        };

        // Read string to lines.
        let request: Vec<_> = request_str.lines().collect();
        // HTTP method in first line.
        let first_line = match request.first() {
            Some(res) => String::from(*res),
            None => {
                error!("failed to parse request method");
                return Err(FrameError::new("failed to parse request method"));
            }
        };

        let headers = collect_headers(&request);

        let mut router: HashMap<&'static str, Option<String>> = HashMap::new();
        let first_line = first_line.clone();
        let mut inline_first_line: Vec<_> =
            first_line.split(' ').map(|txt| String::from(txt)).collect();

        if let Some(method) = inline_first_line.first() {
            router.insert("method", Some(method.clone()));
        }
        if let Some(path) = inline_first_line.get(1) {
            router.insert("path", Some(path.clone()));
        }

        Ok(Self {
            request_str,
            headers,
            first_line,
            router,
        })
    }
}

/// Read http request to string.
fn read_request(reader: &mut BufReader<&mut &TcpStream>) -> Result<String> {
    let mut request_string = String::new();
    loop {
        let byte = reader.read_line(&mut request_string)?;
        if byte < 3 {
            break;
        }
    }
    Ok(request_string)
}

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
