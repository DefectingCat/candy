---
slug: config-watcher-implementation
title: Config Hot Reload Mechanism in Candy Server
authors: [xfy]
tags: [candy, rust, configuration, hot-reload, async]
---

# Config Hot Reload Mechanism in Candy Server

## Introduction

In modern server development, config hot reload is a very important feature. It allows the server to dynamically load updated configurations at runtime without restarting the entire service, significantly improving system availability and maintenance efficiency. As a modern web server written in Rust, Candy provides a powerful and stable config hot reload feature.

This article will deeply analyze the implementation details of the config hot reload mechanism in Candy server, based on the code in `src/utils/config_watcher.rs`.

## Core Components and Design

### 1. ConfigWatcherConfig Struct

```rust
#[derive(Debug, Clone)]
pub struct ConfigWatcherConfig {
    pub debounce_ms: u64,                // Debounce time
    pub rewatch_delay_ms: u64,           // Wait time after rename/delete events
    pub max_retries: usize,              // Maximum retry count
    pub retry_delay_ms: u64,             // Retry delay
    pub poll_timeout_secs: u64,          // Event listening timeout
}
```

These configuration parameters are carefully designed to ensure stability and reliability of the config watching process:

- **Debounce mechanism**: Prevents frequent file system events from causing duplicate processing
- **Rename/delete event handling**: Ensures file operations are complete before processing
- **Retry mechanism**: Automatically retries when reading config or re-watching fails
- **Timeout handling**: Prevents the listening process from completely blocking

### 2. Startup Function Interfaces

Candy provides two functions to start the config watcher:

```rust
// Simplified version (backward compatible)
pub fn start_config_watcher(
    config_path: impl AsRef<Path>,
    callback: impl Fn(Result<Settings>) -> futures::future::BoxFuture<'static, ()>
) -> Result<oneshot::Sender<()>, notify::Error>

// Version with config parameters (recommended)
pub fn start_config_watcher_with_config(
    config_path: impl AsRef<Path>,
    callback: impl Fn(Result<Settings>) -> futures::future::BoxFuture<'static, ()>,
    watcher_config: Option<ConfigWatcherConfig>,
) -> Result<oneshot::Sender<()>, notify::Error>
```

Both functions return a `oneshot::Sender<()>` for sending stop signals to implement graceful shutdown.

## Deep Dive into Implementation Principles

### 1. Event Listening Architecture

Uses `notify` library's `recommended_watcher` to create a default event listener and convert synchronous callbacks to async sending:

```rust
let watcher = std::sync::Arc::new(std::sync::Mutex::new(Box::new(notify::recommended_watcher(
    move |res| {
        let _ = tx.try_send(res);
    },
)?)) as Box<dyn Watcher + Send>);
```

### 2. Core Event Loop

```rust
loop {
    tokio::select! {
        _ = &mut stop_rx => {
            info!("Stopping config watcher");
            break;
        }
        
        result = rx.recv() => {
            // Handle file system events
        }
        
        _ = time::sleep(poll_timeout) => continue,
    }
}
```

The event loop uses Tokio's `select!` macro to implement:
- Stop signal listening
- File system event reception
- Timeout checking (to prevent complete blocking)

### 3. Event Handling Logic

#### Event Relevance Judgment

```rust
fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(notify::event::ModifyKind::Data(_)) // File content changes
        | EventKind::Modify(notify::event::ModifyKind::Name(_)) // File rename
        | EventKind::Remove(_) // File deletion
        | EventKind::Create(_) // File creation
    )
}
```

Only handles events related to config changes to avoid unnecessary processing.

#### Debounce Mechanism

```rust
let now = Instant::now();
if now.duration_since(last_event_time) > debounce_duration {
    info!("Config file event: {:?}", event);
    handle_config_change(...).await;
    last_event_time = now;
}
```

Prevents duplicate triggering of config reload in a short period of time.

### 4. Config Change Handling

```rust
async fn handle_config_change(
    config_path: &std::path::Path,
    watcher: std::sync::Arc<std::sync::Mutex<Box<dyn Watcher + Send>>>,
    callback: std::sync::Arc<ConfigChangeCallback>,
    config: &ConfigWatcherConfig,
    event_kind: EventKind,
) {
    let needs_re_watch = needs_re_watch(event_kind);

    if needs_re_watch {
        time::sleep(Duration::from_millis(config.rewatch_delay_ms)).await;
    }

    let result = retry_with_delay(
        config.max_retries,
        Duration::from_millis(config.retry_delay_ms),
        || Settings::new(config_str),
    ).await;

    if needs_re_watch {
        tokio::task::spawn_blocking(move || {
            retry_with_delay_sync(...)
        }).await;
    }

    callback(result).await;
}
```

#### Retry Mechanism

Provides both async and sync retry functions:

```rust
// Async retry
async fn retry_with_delay<T, E, F>(
    max_retries: usize,
    delay: Duration,
    mut operation: F,
) -> Result<T, E>

// Sync retry
fn retry_with_delay_sync<T, E, F>(
    max_retries: usize,
    delay: std::time::Duration,
    mut operation: F,
) -> Result<T, E>
```

The retry mechanism ensures recovery capability when file reading fails.

## Architectural Design Advantages

### 1. Async Design

Uses Tokio async runtime to achieve high concurrency and efficient event processing.

### 2. Error Handling

Comprehensive error handling and recovery mechanisms, including:
- Retry mechanism
- Error logging
- Async task error capture

### 3. Thread Safety

Uses `Arc<Mutex<T>>` to ensure safe cross-task access to the watcher.

### 4. Robust Design

- **Debounce**: Prevents frequent file system events
- **Re-watch**: Handles file rename/delete events
- **Timeout check**: Prevents blocking

## Usage Example

```rust
use candy::config::Settings;
use candy::utils::config_watcher::{start_config_watcher, ConfigWatcherConfig};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_path = "config.toml";
    let (stop_tx, _) = start_config_watcher(
        config_path,
        |result| Box::pin(async move {
            match result {
                Ok(settings) => {
                    println!("New config loaded: {:?}", settings);
                    // Update server config here
                }
                Err(e) => {
                    eprintln!("Failed to load config: {:?}", e);
                }
            }
        }),
    ).expect("Failed to start config watcher");

    // Server running normally...

    // When needing to stop
    drop(stop_tx);
}
```

## Performance Optimization Strategies

### 1. Event Filtering

Only handles events related to config changes, reducing unnecessary processing.

### 2. Async Channels

Uses Tokio's `mpsc` channels for async event delivery.

### 3. Debounce Mechanism

Prevents repeated triggering of config reload in a short period.

### 4. Task Separation

Separates blocking operations (such as file reading) from async tasks to ensure responsiveness.

## Summary

The config hot reload mechanism in Candy server is a well-designed system that fully embodies best practices of modern Rust async programming. It provides:

- **Stability**: Retry mechanism and error recovery
- **Reliability**: Debounce and timeout handling
- **Ease of use**: Simple API interface
- **Configurability**: Detailed parameter adjustments
- **Performance**: Async design and efficient event processing

This implementation demonstrates how to correctly handle file system events in complex systems and ensures atomicity and consistency of config change processes. It serves as an excellent reference implementation for any Rust application that needs config hot reload functionality.
