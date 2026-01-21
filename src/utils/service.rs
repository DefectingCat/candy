use tracing::debug;

/// Parse port from host
/// if host is localhost:8080
/// return 8080
/// if host is localhost
/// return 80
pub fn parse_port_from_host(host: &str, scheme: &str) -> Option<u16> {
    // localhost:8080
    // ["localhost", "8080"]
    // localhost
    // ["localhost"]
    let host_parts = host.split(':').collect::<Vec<&str>>();
    let port = if host_parts.len() == 1 {
        match scheme {
            "http" => 80,
            "https" => 443,
            _ => {
                debug!("scheme not support");
                return None;
            }
        }
    } else {
        host_parts.get(1)?.parse::<u16>().ok()?
    };
    Some(port)
}
