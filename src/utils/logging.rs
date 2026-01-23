use std::str::FromStr;

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
    let env_layer = EnvFilter::from_str(log_level).unwrap_or_else(|_| "info".into());
    let is_debug = log_level.to_lowercase().contains("debug");

    // 控制台输出格式化层
    let mut console_layer_builder = fmt::layer().with_target(false).with_writer(std::io::stdout);
    if is_debug {
        console_layer_builder = console_layer_builder
            .with_file(true)
            .with_line_number(true)
            .with_target(true);
    }
    let formatting_layer = console_layer_builder;

    // 使用 builder 模式创建 RollingFileAppender，这样可以捕获初始化错误
    let builder = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("candy_log");

    match builder.build(log_folder) {
        Ok(file_appender) => {
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let mut file_layer_builder = fmt::layer()
                .compact()
                .with_target(false)
                .with_thread_ids(true)
                .with_ansi(false)
                .with_writer(non_blocking);
            if is_debug {
                file_layer_builder = file_layer_builder.with_file(true).with_line_number(true);
            }
            let file_subscriber = file_layer_builder;

            let collector = tracing_subscriber::registry()
                .with(env_layer)
                .with(formatting_layer)
                .with(file_subscriber);

            // 尝试设置全局默认订阅器，如果已设置则不报错
            if tracing::subscriber::set_global_default(collector).is_err() {
                // 如果订阅器已设置，我们可以继续，只是不会使用文件输出
                // 创建一个 dummy guard 来保持接口一致
                let dummy_appender = tracing_appender::rolling::never("/dev/null", "dummy");
                let (_, dummy_guard) = tracing_appender::non_blocking(dummy_appender);
                Ok(dummy_guard)
            } else {
                Ok(guard)
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to initialize log file appender ({:?}), will only output logs to console",
                e
            );

            let collector = tracing_subscriber::registry()
                .with(env_layer)
                .with(formatting_layer);

            // 尝试设置全局默认订阅器，如果已设置则不报错
            match tracing::subscriber::set_global_default(collector) {
                Err(_) => {
                    let dummy_appender = tracing_appender::rolling::never("/dev/null", "dummy");
                    let (_, dummy_guard) = tracing_appender::non_blocking(dummy_appender);
                    Ok(dummy_guard)
                }
                Ok(_) => {
                    // 创建一个 dummy guard，因为我们需要返回一个值
                    let dummy_appender = tracing_appender::rolling::RollingFileAppender::builder()
                        .rotation(tracing_appender::rolling::Rotation::NEVER)
                        .filename_prefix("dummy")
                        .build("/tmp") // /tmp 目录通常是可写的
                        .unwrap_or_else(|_| {
                            // 如果 /tmp 也不可写，那么我们只能创建一个内存 appender 或者直接返回一个空 guard
                            // 这里我们使用 never rotate 到 /dev/null，这在 Unix 系统上应该总是可行的
                            tracing_appender::rolling::never("/dev/null", "dummy")
                        });
                    let (_, dummy_guard) = tracing_appender::non_blocking(dummy_appender);

                    Ok(dummy_guard)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logger_with_invalid_log_level() {
        // 使用一个不会创建文件的路径，或者直接测试逻辑
        let guard = init_logger("invalid_level", "/dev/null");
        assert!(guard.is_ok());

        let _ = guard.unwrap();
    }
}
