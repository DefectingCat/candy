use std::str::FromStr;

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
pub fn init_logger(log_level: &str, log_folder: &str) {
    let file_appender = tracing_appender::rolling::daily(log_folder, "candy_log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // let file_subscriber = fmt::layer().with_writer(non_blocking).with_ansi(false); // 禁用 ANSI 颜色，因为日志文件不需要颜色
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
    tracing::subscriber::set_global_default(collector).expect("Unable to set a global collector");
}
