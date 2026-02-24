//! 测试数据准备

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn create_test_directory() -> Result<(PathBuf, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let temp_dir_path = temp_dir.path().to_path_buf();
    let index_html_path = temp_dir_path.join("index.html");
    let test_content = "<html><body>Test Page</body></html>";
    
    fs::write(index_html_path.clone(), test_content)?;
    
    let _ = temp_dir.keep(); // 在使用完 temp_dir 之后再调用
    
    println!("Test directory created at: {}", temp_dir_path.display());
    println!("Test index.html created at: {}", index_html_path.display());
    
    Ok((temp_dir_path, index_html_path))
}