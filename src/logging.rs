use chrono::{DateTime, Utc};
use std::io::Write;

use crate::config::LogFormat;

/// 访问日志记录
#[derive(Debug)]
pub struct AccessLog {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 客户端地址
    pub client_addr: String,
    /// 请求方法
    pub method: String,
    /// 请求路径
    pub path: String,
    /// HTTP 版本
    pub version: String,
    /// 响应状态码
    pub status: u16,
    /// 响应大小（字节）
    pub response_size: usize,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// Referer 头
    pub referer: Option<String>,
    /// User-Agent 头
    pub user_agent: Option<String>,
}

impl AccessLog {
    /// 创建新的访问日志
    pub fn new(
        client_addr: String,
        method: String,
        path: String,
        version: String,
        status: u16,
        response_size: usize,
        response_time_ms: u64,
    ) -> Self {
        AccessLog {
            timestamp: Utc::now(),
            client_addr,
            method,
            path,
            version,
            status,
            response_size,
            response_time_ms,
            referer: None,
            user_agent: None,
        }
    }

    /// 设置 Referer
    pub fn with_referer(mut self, referer: Option<String>) -> Self {
        self.referer = referer;
        self
    }

    /// 设置 User-Agent
    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// 格式化为 Combined 日志格式
    /// 格式: $client_addr - - [$timestamp] "$method $path $version" $status $response_size "$referer" "$user_agent" $response_time_ms
    pub fn to_combined(&self) -> String {
        let timestamp = self.timestamp.format("%d/%b/%Y:%H:%M:%S %z");
        let referer = self.referer.as_deref().unwrap_or("-");
        let user_agent = self.user_agent.as_deref().unwrap_or("-");

        format!(
            "{} - - [{}] \"{} {} {}\" {} {} \"{}\" \"{}\" {}",
            self.client_addr,
            timestamp,
            self.method,
            self.path,
            self.version,
            self.status,
            self.response_size,
            referer,
            user_agent,
            self.response_time_ms
        )
    }

    /// 格式化为 JSON
    pub fn to_json(&self) -> String {
        let timestamp = self.timestamp.to_rfc3339();
        let referer = self.referer.as_deref().unwrap_or("-");
        let user_agent = self.user_agent.as_deref().unwrap_or("-");

        format!(
            "{{\"timestamp\":\"{}\",\"client_addr\":\"{}\",\"method\":\"{}\",\"path\":\"{}\",\"version\":\"{}\",\"status\":{},\"response_size\":{},\"response_time_ms\":{},\"referer\":\"{}\",\"user_agent\":\"{}\"}}",
            timestamp,
            self.client_addr,
            self.method,
            self.path,
            self.version,
            self.status,
            self.response_size,
            self.response_time_ms,
            referer,
            user_agent
        )
    }

    /// 根据格式输出日志
    pub fn format(&self, format: LogFormat) -> String {
        match format {
            LogFormat::Combined => self.to_combined(),
            LogFormat::Json => self.to_json(),
        }
    }
}

/// 日志写入器
pub struct Logger {
    format: LogFormat,
    output: Box<dyn Write + Send>,
}

impl Logger {
    /// 创建标准输出日志器
    pub fn stdout(format: LogFormat) -> Self {
        Logger {
            format,
            output: Box::new(std::io::stdout()),
        }
    }

    /// 创建文件日志器
    pub fn file(format: LogFormat, path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Logger {
            format,
            output: Box::new(file),
        })
    }

    /// 记录访问日志
    pub fn log_access(&mut self, log: &AccessLog) -> std::io::Result<()> {
        let formatted = log.format(self.format);
        self.output.write_all(formatted.as_bytes())?;
        self.output.write_all(b"\n")?;
        self.output.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_log_new() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/index.html".to_string(),
            "HTTP/1.1".to_string(),
            200,
            1024,
            50,
        );
        assert_eq!(log.method, "GET");
        assert_eq!(log.status, 200);
        assert_eq!(log.response_size, 1024);
        assert_eq!(log.response_time_ms, 50);
    }

    #[test]
    fn test_access_log_with_referer() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/".to_string(),
            "HTTP/1.1".to_string(),
            200,
            100,
            10,
        ).with_referer(Some("http://example.com".to_string()));
        assert_eq!(log.referer, Some("http://example.com".to_string()));
    }

    #[test]
    fn test_access_log_with_user_agent() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/".to_string(),
            "HTTP/1.1".to_string(),
            200,
            100,
            10,
        ).with_user_agent(Some("Mozilla/5.0".to_string()));
        assert_eq!(log.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_access_log_to_combined() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/index.html".to_string(),
            "HTTP/1.1".to_string(),
            200,
            1024,
            50,
        );
        let combined = log.to_combined();
        assert!(combined.contains("127.0.0.1:8080"));
        assert!(combined.contains("GET /index.html HTTP/1.1"));
        assert!(combined.contains("200"));
        assert!(combined.contains("1024"));
        assert!(combined.contains("50"));
    }

    #[test]
    fn test_access_log_to_json() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/index.html".to_string(),
            "HTTP/1.1".to_string(),
            200,
            1024,
            50,
        );
        let json = log.to_json();
        assert!(json.contains("\"method\":\"GET\""));
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"response_size\":1024"));
        assert!(json.contains("\"response_time_ms\":50"));
    }

    #[test]
    fn test_log_format_combined() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/".to_string(),
            "HTTP/1.1".to_string(),
            200,
            100,
            10,
        );
        let formatted = log.format(LogFormat::Combined);
        assert!(formatted.starts_with("127.0.0.1:8080"));
    }

    #[test]
    fn test_log_format_json() {
        let log = AccessLog::new(
            "127.0.0.1:8080".to_string(),
            "GET".to_string(),
            "/".to_string(),
            "HTTP/1.1".to_string(),
            200,
            100,
            10,
        );
        let formatted = log.format(LogFormat::Json);
        assert!(formatted.starts_with('{'));
        assert!(formatted.ends_with('}'));
    }
}