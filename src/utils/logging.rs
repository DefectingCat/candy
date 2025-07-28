use tracing_subscriber::{
    EnvFilter,
    fmt::{self},
    prelude::*,
    registry,
};

/// 初始化 Logger
///
/// 从配置文件中读取 log 级别，同时读取日志文件存储路径。
/// 无论是否设置了日志文件路径，都会将日志输出到控制台。
///
/// 配置文件路径只能文件夹，日志文件将按天分割。
pub fn init_logger() {
    let formatting_layer = fmt::layer()
        // .pretty()
        // .with_thread_ids(true)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::try_from_env("CANDY_LOG").unwrap_or_else(|_| "info".into());
    registry().with(env_layer).with(formatting_layer).init();
}
