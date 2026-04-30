use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;

use crate::config::TlsConfig;

/// TLS 配置错误
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Certificate parse error: {0}")]
    CertificateParse(String),

    #[error("Private key parse error: {0}")]
    KeyParse(String),

    #[error("No valid certificate found")]
    NoCertificate,

    #[error("No valid private key found")]
    NoPrivateKey,
}

/// 加载 TLS 服务器配置
pub fn load_tls_config(config: &TlsConfig) -> Result<Arc<ServerConfig>, TlsError> {
    // 加载证书链
    let certs = load_certs(&config.cert)?;

    // 加载私钥
    let key = load_private_key(&config.key)?;

    // 构建 ServerConfig
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::CertificateParse(e.to_string()))?;

    Ok(Arc::new(server_config))
}

/// 从 PEM 文件加载证书链
fn load_certs(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut reader)
        .filter_map(|result| result.ok())
        .collect();

    if certs.is_empty() {
        return Err(TlsError::NoCertificate);
    }

    Ok(certs)
}

/// 从 PEM 文件加载私钥
fn load_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // 尝试加载 PKCS#8 私钥
    if let Some(key) = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| TlsError::KeyParse(e.to_string()))?
    {
        return Ok(key);
    }

    Err(TlsError::NoPrivateKey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_certs_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = load_certs(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::NoCertificate));
    }

    #[test]
    fn test_load_private_key_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = load_private_key(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::NoPrivateKey));
    }

    #[test]
    fn test_load_tls_config_missing_cert() {
        let config = TlsConfig {
            enabled: true,
            cert: std::path::PathBuf::from("/nonexistent/cert.pem"),
            key: std::path::PathBuf::from("/nonexistent/key.pem"),
        };
        let result = load_tls_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_certs_valid_cert() {
        // 使用预生成的测试证书
        let cert_path = std::path::PathBuf::from("test_data/test_cert.pem");
        if cert_path.exists() {
            let result = load_certs(&cert_path);
            assert!(result.is_ok());
            let certs = result.unwrap();
            assert!(!certs.is_empty());
        }
    }

    #[test]
    fn test_load_private_key_valid_key() {
        // 使用预生成的测试私钥
        let key_path = std::path::PathBuf::from("test_data/test_key.pem");
        if key_path.exists() {
            let result = load_private_key(&key_path);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_load_tls_config_valid() {
        let cert_path = std::path::PathBuf::from("test_data/test_cert.pem");
        let key_path = std::path::PathBuf::from("test_data/test_key.pem");

        if cert_path.exists() && key_path.exists() {
            let config = TlsConfig {
                enabled: true,
                cert: cert_path,
                key: key_path,
            };
            let result = load_tls_config(&config);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::NoCertificate;
        assert_eq!(err.to_string(), "No valid certificate found");

        let err = TlsError::NoPrivateKey;
        assert_eq!(err.to_string(), "No valid private key found");
    }
}
