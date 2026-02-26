use std::sync::OnceLock;
use std::str::FromStr;

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter,
    filter::LevelFilter,
    fmt::{self},
    layer::SubscriberExt,
};

/// 全局标志，用于跟踪 logger 是否已初始化
static LOGGER_INITIALIZED: OnceLock<()> = OnceLock::new();

/// 创建 dummy guard 用于保持接口一致性
fn create_dummy_guard() -> WorkerGuard {
    let (_, dummy_guard) = tracing_appender::non_blocking(std::io::sink());
    dummy_guard
}

/// 尝试初始化 Logger（如果尚未初始化）
///
/// 此函数是线程安全的，可以多次调用。
/// 如果 logger 已经初始化，则返回一个 dummy guard。
///
/// # 参数
///
/// * `log_level` - 日志级别字符串（如 "debug", "info", "warn", "error"）
/// * `log_folder` - 日志文件存储路径，为空或 "/dev/null" 则只输出到控制台
///
/// # 返回值
///
/// 返回 `WorkerGuard`，用于保持日志文件写入器的生命周期
pub fn try_init_logger(log_level: &str, log_folder: &str) -> anyhow::Result<WorkerGuard> {
    // 如果已经初始化，直接返回 dummy guard
    if LOGGER_INITIALIZED.get().is_some() {
        return Ok(create_dummy_guard());
    }

    // 进行实际初始化
    let result = init_logger_impl(log_level, log_folder);

    // 无论成功与否，都标记为已尝试初始化
    // 这样可以避免在多线程环境下重复尝试
    let _ = LOGGER_INITIALIZED.set(());

    result
}

/// 内部实现：初始化 Logger
fn init_logger_impl(log_level: &str, log_folder: &str) -> anyhow::Result<WorkerGuard> {
    // 先严格验证日志级别是否有效
    let _ = LevelFilter::from_str(log_level)
        .with_context(|| format!("Invalid log level: {}", log_level))?;

    let env_layer = EnvFilter::from_str(log_level)
        .with_context(|| format!("Invalid log level: {}", log_level))?;
    let is_debug = log_level.to_lowercase().contains("debug");

    // 控制台输出格式化层
    let mut console_layer_builder = fmt::layer().with_target(false).with_writer(std::io::stdout);
    if is_debug {
        console_layer_builder = console_layer_builder
            .with_file(true)
            .with_line_number(true)
            .with_target(true);
    }
    let console_layer = console_layer_builder;

    // 尝试添加文件输出层（如果配置了有效的日志文件夹）
    if !log_folder.is_empty() && log_folder != "/dev/null" {
        match tracing_appender::rolling::Builder::new()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("candy.log")
            .build(log_folder)
        {
            Ok(file_appender) => {
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                let file_layer = fmt::layer().with_target(true).with_writer(non_blocking);

                let collector = tracing_subscriber::registry()
                    .with(env_layer)
                    .with(console_layer)
                    .with(file_layer);

                if tracing::subscriber::set_global_default(collector).is_err() {
                    Ok(create_dummy_guard())
                } else {
                    Ok(guard)
                }
            }
            Err(_) => {
                // 文件输出失败，只使用控制台输出
                let collector = tracing_subscriber::registry()
                    .with(env_layer)
                    .with(console_layer);

                let _ = tracing::subscriber::set_global_default(collector);
                Ok(create_dummy_guard())
            }
        }
    } else {
        // 未配置日志文件夹或使用 /dev/null，只使用控制台输出
        let collector = tracing_subscriber::registry()
            .with(env_layer)
            .with(console_layer);

        let _ = tracing::subscriber::set_global_default(collector);
        Ok(create_dummy_guard())
    }
}

/// 初始化 Logger
///
/// 从配置文件中读取 log 级别，同时读取日志文件存储路径。
/// 无论是否设置了日志文件路径，都会将日志输出到控制台。
///
/// 配置文件路径只能是文件夹，日志文件将按天分割。
///
/// 注意：此函数在整个进程生命周期中只能成功调用一次。
/// 如果需要多次调用（如在测试中），请使用 `try_init_logger`。
pub fn init_logger(log_level: &str, log_folder: &str) -> anyhow::Result<WorkerGuard> {
    try_init_logger(log_level, log_folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logger_with_invalid_log_level() {
        let guard = init_logger("invalid_level", "/dev/null");
        assert!(guard.is_err());
    }

    #[test]
    fn test_init_logger_with_valid_config() {
        let guard = init_logger("info", "/dev/null");
        assert!(guard.is_ok());
    }

    #[test]
    fn test_init_logger_with_debug_level() {
        let guard = init_logger("debug", "/dev/null");
        assert!(guard.is_ok());
    }
}
