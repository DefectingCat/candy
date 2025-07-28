use std::str::FromStr;

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self},
    layer::SubscriberExt,
};

/// 初始化 Logger
///
/// 从配置文件中读取 log 级别，同时读取日志文件存储路径。
/// 无论是否设置了日志文件路径，都会将日志输出到控制台。
///
/// 配置文件路径只能文件夹，日志文件将按天分割。
pub fn init_logger(log_level: &str, log_folder: &str) -> anyhow::Result<WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily(log_folder, "candy_log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_subscriber = fmt::layer()
        .compact()
        .with_target(false)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_writer(non_blocking);

    let formatting_layer = fmt::layer()
        // .pretty()
        // .with_thread_ids(true)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::from_str(log_level).unwrap_or_else(|_| "info".into());

    let collector = tracing_subscriber::registry()
        .with(env_layer)
        .with(formatting_layer)
        .with(file_subscriber);
    tracing::subscriber::set_global_default(collector)
        .with_context(|| "to set a global collector")?;
    Ok(guard)
}
