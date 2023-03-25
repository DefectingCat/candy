use std::collections::HashMap;

use anyhow::Result;
use log::error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

use crate::error::CandyError;

pub struct HttpFrame {
    pub request_str: String,
    pub headers: HashMap<String, String>,
    pub router: HashMap<&'static str, Option<String>>,
}

impl HttpFrame {
    pub async fn build(reader: BufReader<&mut TcpStream>) -> Result<Self, CandyError> {
        let request_str = match read_request(reader).await {
            Ok(str) => str,
            Err(err) => {
                error!("{:?}", err);
                return Err(CandyError::Parse(err.to_string()));
            }
        };

        // Read string to lines.
        let request: Vec<_> = request_str.lines().collect();
        let headers = collect_headers(&request);

        // HTTP method in first line.
        let first_line = match request.first() {
            Some(res) => (*res).to_string(),
            None => {
                error!("failed to parse request method");
                return Err(CandyError::Parse(
                    "failed to parse request method".to_string(),
                ));
            }
        };
        let mut router: HashMap<&'static str, Option<String>> = HashMap::new();
        let inline_first_line: Vec<_> = first_line.split(' ').collect();
        let method = if let Some(m) = inline_first_line.first() {
            Some((**m).to_string())
        } else {
            None
        };
        let path = if let Some(p) = inline_first_line.get(1) {
            Some((**p).to_string())
        } else {
            None
        };
        router.insert("method", method);
        router.insert("path", path);

        Ok(Self {
            request_str,
            headers,
            router,
        })
    }
}

/// Read http request to string.
async fn read_request(mut reader: BufReader<&mut TcpStream>) -> Result<String> {
    let mut request_string = String::new();
    loop {
        let byte = reader.read_line(&mut request_string).await?;
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
