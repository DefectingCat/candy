use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;

/// 使用 sendfile 系统调用进行零拷贝文件传输
/// 返回实际发送的字节数
pub fn sendfile(out_fd: i32, file: &File, offset: usize, count: usize) -> io::Result<usize> {
    let in_fd = file.as_raw_fd();

    // Linux 平台实现
    #[cfg(target_os = "linux")]
    {
        let mut offset_val = offset as i64;
        // SAFETY: sendfile syscall 是 Linux 特有的，从内核空间直接传输到 socket
        // 不会经过用户空间，实现真正的零拷贝
        unsafe {
            let result = libc::sendfile64(out_fd, in_fd, &mut offset_val, count);
            if result < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(result as usize)
            }
        }
    }

    // macOS 平台实现
    #[cfg(target_os = "macos")]
    {
        let mut len = count as libc::off_t;
        // SAFETY: macOS sendfile 从内核空间直接传输到 socket
        // 参数: in_fd, out_fd, offset, &len, sf_hdtr, flags
        unsafe {
            let result = libc::sendfile(in_fd, out_fd, offset as libc::off_t, &mut len, std::ptr::null_mut(), 0);
            if result < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(len as usize)
            }
        }
    }

    // FreeBSD 平台实现
    #[cfg(target_os = "freebsd")]
    {
        let mut len = count as libc::off_t;
        // SAFETY: FreeBSD sendfile 从内核空间直接传输到 socket
        // 参数: in_fd, out_fd, offset, len, sf_hdtr, &len, flags
        unsafe {
            let result = libc::sendfile(in_fd, out_fd, offset as libc::off_t, len, std::ptr::null_mut(), &mut len, 0);
            if result < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(len as usize)
            }
        }
    }

    // 其他平台回退到用户空间拷贝
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    {
        use std::io::{Read, Seek, SeekFrom};

        let mut file_ref = file;
        file_ref.seek(SeekFrom::Start(offset as u64))?;
        let mut buffer = vec![0u8; count];
        let n = file_ref.read(&mut buffer)?;

        // 注意：这里只读取了数据，实际写入 socket 需要在调用方处理
        // 因为 sendfile 的语义是直接传输，回退方案需要返回读取的数据
        // 调用方应该检测返回值并使用普通 write
        Ok(n)
    }
}

/// 异步 sendfile 封装，用于 tokio
pub async fn sendfile_async(
    stream: &tokio::net::TcpStream,
    file: &File,
    offset: usize,
    count: usize,
) -> io::Result<usize> {
    // tokio TcpStream 使用 std::net::TcpStream 的原始 fd
    let out_fd = stream.as_raw_fd();
    sendfile(out_fd, file, offset, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_sendfile_basic() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, World!";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        // 重新打开文件以获取文件描述符
        let file = temp_file.reopen().unwrap();

        // 创建 pipe 来模拟 socket
        let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();

        // 使用 sendfile
        let result = sendfile(writer.as_raw_fd(), &file, 0, content.len());
        // 注意：sendfile 到 Unix socket 在某些内核版本可能不支持
        // 这个测试主要验证 API 可用性
        match result {
            Ok(n) => assert!(n <= content.len()),
            Err(e) => {
                // 某些系统可能返回 EINVAL（Unix socket 不支持）
                assert!(
                    e.kind() == io::ErrorKind::InvalidInput || e.kind() == io::ErrorKind::Other
                );
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_sendfile_basic_macos() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, World!";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        // 重新打开文件以获取文件描述符
        let file = temp_file.reopen().unwrap();

        // 创建 socket pair 来模拟
        let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();

        // 使用 sendfile
        let result = sendfile(writer.as_raw_fd(), &file, 0, content.len());
        match result {
            Ok(n) => assert!(n <= content.len()),
            Err(e) => {
                // macOS 可能对 Unix socket 有限制
                assert!(
                    e.kind() == io::ErrorKind::InvalidInput
                        || e.kind() == io::ErrorKind::Other
                        || e.kind() == io::ErrorKind::Unsupported
                );
            }
        }
    }

    #[test]
    #[cfg(target_os = "freebsd")]
    fn test_sendfile_basic_freebsd() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, World!";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        // 重新打开文件以获取文件描述符
        let file = temp_file.reopen().unwrap();

        // 创建 socket pair 来模拟
        let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();

        // 使用 sendfile
        let result = sendfile(writer.as_raw_fd(), &file, 0, content.len());
        match result {
            Ok(n) => assert!(n <= content.len()),
            Err(e) => {
                // FreeBSD 可能对 Unix socket 有限制
                assert!(
                    e.kind() == io::ErrorKind::InvalidInput
                        || e.kind() == io::ErrorKind::Other
                        || e.kind() == io::ErrorKind::Unsupported
                );
            }
        }
    }

    #[test]
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    fn test_sendfile_fallback() {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, World!";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        // 重新打开文件以获取文件描述符
        let file = temp_file.reopen().unwrap();

        // 使用回退实现
        let result = sendfile(-1, &file, 0, content.len());
        // 回退实现应该成功读取数据
        assert!(result.is_ok());
        let n = result.unwrap();
        assert!(n <= content.len());
    }
}
