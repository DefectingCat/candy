use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::socket::create_reuseport_listener;

/// Worker 进程主函数
pub fn run(config: &Config) -> std::io::Result<()> {
    println!(
        "Worker {} starting on {}",
        std::process::id(),
        config.server.listen
    );

    // 创建 tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        // 创建 SO_REUSEPORT listener
        let listener = create_reuseport_listener(config.server.listen)?;

        println!("Worker {} listening on {}", std::process::id(), config.server.listen);

        // 主循环
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    // 处理连接（暂时只打印）
                    tokio::spawn(async move {
                        let _ = handle_connection(stream, addr).await;
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    })
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
) -> std::io::Result<()> {
    // 暂时只打印连接信息
    println!("Connection from {}", addr);

    // 关闭连接
    stream.shutdown().await?;
    Ok(())
}
