use std::net::SocketAddr;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::{io, net};

use nix::sys::socket::{
    bind, listen, setsockopt, socket, sockopt::ReusePort, AddressFamily, Backlog, SockFlag,
    SockType,
};
use tokio::net::TcpListener;

/// 创建支持 SO_REUSEPORT 的 TCP listener
///
/// SO_REUSEPORT 允许多个进程绑定同一端口，内核负责负载均衡分发连接
pub fn create_reuseport_listener(addr: SocketAddr) -> io::Result<TcpListener> {
    // 创建 socket
    let family = if addr.is_ipv6() {
        AddressFamily::Inet6
    } else {
        AddressFamily::Inet
    };

    let sock = socket(family, SockType::Stream, SockFlag::empty(), None)
        .map_err(nix_to_io_error)?;

    // 设置 SO_REUSEPORT
    setsockopt(&sock, ReusePort, &true).map_err(nix_to_io_error)?;

    // 设置 SO_REUSEADDR (允许快速重启)
    setsockopt(&sock, nix::sys::socket::sockopt::ReuseAddr, &true)
        .map_err(nix_to_io_error)?;

    // 绑定地址 - nix 0.31 需要使用 SockaddrStorage
    let nix_addr: nix::sys::socket::SockaddrStorage = addr.into();
    bind(sock.as_raw_fd(), &nix_addr).map_err(nix_to_io_error)?;

    // 开始监听 (backlog = 1024)
    listen(&sock, Backlog::new(1024).map_err(nix_to_io_error)?).map_err(nix_to_io_error)?;

    // 获取原始 fd
    let fd: RawFd = sock.into_raw_fd();

    // 转换为 std::net::TcpListener
    let std_listener = unsafe { net::TcpListener::from_raw_fd(fd) };

    // 设置非阻塞
    std_listener.set_nonblocking(true)?;

    // 转换为 tokio TcpListener
    TcpListener::from_std(std_listener)
}

fn nix_to_io_error(err: nix::errno::Errno) -> io::Error {
    io::Error::from_raw_os_error(err as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_create_listener() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 18080);
        let listener = create_reuseport_listener(addr).unwrap();

        // 验证 local_addr
        assert!(listener.local_addr().is_ok());
    }

    #[tokio::test]
    async fn test_reuseport_multiple_bind() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 18081);

        // 创建两个 listener 绑定同一端口
        let listener1 = create_reuseport_listener(addr).unwrap();
        let listener2 = create_reuseport_listener(addr).unwrap();

        // 两个都应该成功创建
        assert!(listener1.local_addr().is_ok());
        assert!(listener2.local_addr().is_ok());
    }
}
