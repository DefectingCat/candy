---
name: m16-documentation
description: "Use for writing Rust documentation comments. Keywords: 注释, comment, 文档, document, 添加注释, 写注释, add comment, doc comment, rustdoc"
source: https://rust-coding-guidelines.github.io/rust-coding-guidelines-zh/
user-invocable: false
---

# Rust 文档注释规范

## 快速参考

| 类型 | 符号 | 用途 | 必需性 |
|------|------|------|--------|
| 模块文档 | `//!` | crate 根、模块顶部 | 推荐 |
| 项目文档 | `///` | 公开 API | **必需** |
| 实现注释 | `//` | 复杂逻辑 | 按需 |

## 核心规则

### 1. 公开 API 必须文档化

所有 `pub` 项目都需要 `///` 文档注释：

```rust
// ❌ 错误
pub fn process(data: &str) -> Result<()> { ... }

// ✅ 正确
/// 处理输入数据并执行转换。
///
/// # Errors
///
/// 数据格式无效时返回错误。
pub fn process(data: &str) -> Result<()> { ... }
```

### 2. 文档注释结构模板

```rust
/// 简短描述（一行，动词开头，以句号结尾）。
///
/// 详细描述段落（可选）。
/// 可以包含多行，解释功能、用途和注意事项。
///
/// # Arguments
///
/// * `param` - 参数说明
///
/// # Returns
///
/// 返回值说明。
///
/// # Errors
///
/// 错误情况说明（Result 返回类型必需）。
///
/// # Example
///
/// ```
/// use crate::module::Item;
/// let item = Item::new();
/// ```
///
/// # Panics
///
/// panic 条件（可能 panic 时必需）。
///
/// # Safety
///
/// unsafe 代码的安全保证（unsafe 函数必需）。
```

### 3. 模块文档模板

```rust
//! # 模块名称
//!
//! 一句话描述模块功能。
//!
//! ## 功能特性
//!
//! - 特性 1：说明
//! - 特性 2：说明
//!
//! ## 模块结构
//!
//! - [`StructName`]: 结构说明
//! - [`function_name`]: 函数说明
```

### 4. 结构体文档模板

```rust
/// 结构体的简短描述。
///
/// 详细说明用途和使用场景。
///
/// # Example
///
/// ```
/// let item = Item::new();
/// ```
///
/// # Cloning
///
/// （如适用）说明克隆行为和开销。
///
/// # Thread Safety
///
/// （如适用）说明线程安全性。
pub struct Item {
    /// 字段说明（简短即可）。
    pub field: String,
}
```

## 章节规则

### 必需章节

| 章节 | 使用条件 |
|------|----------|
| `# Arguments` | 函数有参数时 |
| `# Returns` | 函数有返回值时 |
| `# Errors` | 返回 `Result` 时 |
| `# Panics` | 可能 panic 时 |
| `# Safety` | `unsafe` 函数时 |

### 可选章节

| 章节 | 使用场景 |
|------|----------|
| `# Example` | 公开 API 推荐 |
| `# Cloning` | 实现了 `Clone` 且行为特殊 |
| `# Thread Safety` | 并发相关 |
| `# Performance` | 性能特征重要时 |
| `# See Also` | 相关项目引用 |

## 实现注释原则

### 何时需要实现注释 (`//`)

1. **复杂逻辑**：非显而易见的算法
2. **性能优化**：为何选择这种实现
3. **边界情况**：特殊处理的条件
4. **锁作用域**：解释作用域限制
5. **TODO/FIXME**：待处理事项

```rust
// 历史长度限制：保留最近 N 轮对话（2N 条消息）
// 使用 split_off 高效截断，避免迭代器开销
if history.len() > self.max_history * 2 {
    *history = history.split_off(history.len() - self.max_history * 2);
}

// 添加用户消息到历史（使用独立作用域限制锁的生命周期）
{
    let mut conv = self.inner.conversation.write().await;
    conv.add_user_message(session_id, prompt);
}
```

### 何时不需要实现注释

```rust
// ❌ 冗余：代码已自解释
let len = items.len();  // 获取长度

// ✅ 必要：解释原因
// 使用 Arc 包装，支持高效的克隆和共享
let shared = Arc::new(inner);
```

## 注释写作规范

### 1. 解释 "为什么"，不是 "是什么"

```rust
// ❌ 差：重复代码含义
// 遍历所有元素
for item in items.iter() { ... }

// ✅ 好：解释意图和原因
// 使用迭代器而非索引，避免边界检查开销
for item in items.iter() { ... }
```

### 2. 保持简洁

```rust
// ❌ 冗长
/// 这个函数用于处理用户的输入数据，它会首先验证数据的格式是否正确，
/// 然后将数据转换成内部格式，最后保存到数据库中。

// ✅ 简洁
/// 处理、验证并保存用户输入数据。
```

### 3. 使用完整句子

```rust
// ❌ 不完整
/// returns the length

// ✅ 完整
/// 返回集合中的元素数量。
```

### 4. 中英文混用规则

- 中文注释：团队成员都用中文
- 英文术语：保留原文（如 `Arc`, `Mutex`, `async`）
- 代码示例：变量名用英文，注释用中文

```rust
/// 使用 `Arc<Mutex<T>>` 实现共享状态。
///
/// 内部使用 `Arc` 包装，支持跨线程克隆共享。
```

## 示例代码规范

### 1. 示例代码格式

```rust
/// # Example
///
/// ```
/// use crate::module::Item;
///
/// let item = Item::new();
/// let result = item.process("data");
/// ```

// 使用 `# ` 隐藏辅助代码
/// # Example
///
/// ```
/// # use crate::module::Item;
/// # let item = Item::new();
/// let result = item.process("data");
/// ```
```

### 2. 示例代码标记

| 标记 | 用途 |
|------|------|
| ```` ``` ```` | 默认：编译并运行测试 |
| ```` ```no_run ```` | 编译但不运行（IO 操作等） |
| ```` ```ignore ```` | 不编译也不运行（不完整示例） |
| ```` ```compile_fail ```` | 应该编译失败（错误示例） |

## 文档链接

### 1. 链接到其他项目

```rust
/// 与 [`chat`](Self::chat) 类似，但支持流式输出。
/// 参考 [`ConversationManager`] 了解会话管理。
pub fn chat_stream(&self, ...) { ... }
```

### 2. 链接语法

```rust
[`Item`]                    // 同模块
[`module::Item`]            // 其他模块
[`Item::method`]            // 方法
[`method`](Item::method)    // 显式路径
```

## 检查清单

### 写注释前检查

- [ ] 是否是公开 API？→ 必须有 `///` 文档
- [ ] 是否返回 `Result`？→ 必须有 `# Errors`
- [ ] 是否可能 panic？→ 必须有 `# Panics`
- [ ] 是否是 unsafe？→ 必须有 `# Safety`
- [ ] 是否有参数？→ 应该有 `# Arguments`
- [ ] 是否有返回值？→ 应该有 `# Returns`

### 写注释时检查

- [ ] 第一行是否是简短描述？
- [ ] 是否解释了 "为什么" 而非 "是什么"？
- [ ] 示例代码是否可编译？
- [ ] 注释是否与代码同步？

### 写注释后检查

```bash
# 生成文档并检查警告
cargo doc --no-deps

# 运行文档测试
cargo test --doc

# 使用 clippy 检查文档问题
cargo clippy -- -W clippy::doc_markdown
```

## 常见错误

### 1. 缺少必需章节

```rust
// ❌ 错误：返回 Result 但没有 # Errors
pub fn load(path: &str) -> Result<Data> { ... }

// ✅ 正确
/// 从文件加载数据。
///
/// # Errors
///
/// 文件不存在或格式无效时返回错误。
pub fn load(path: &str) -> Result<Data> { ... }
```

### 2. 示例代码不可编译

```rust
// ❌ 错误：缺少导入
/// # Example
///
/// ```
/// let item = Item::new();  // 未定义 Item
/// ```

// ✅ 正确
/// # Example
///
/// ```
/// use crate::module::Item;
/// let item = Item::new();
/// ```
```

### 3. 注释与代码不同步

```rust
// ❌ 错误：注释说返回 3，实际返回 4
/// 返回前 3 个元素。
fn top_three(items: &[i32]) -> &[i32] {
    &items[..4]  // 实际返回 4 个
}
```

## 工具支持

```bash
# 检查文档缺失
cargo doc --open

# 运行文档测试
cargo test --doc

# clippy 文档检查
cargo clippy -- -W clippy::missing_docs_in_private_items

# rustdoc 配置（lib.rs）
#![warn(missing_docs)]
```

## 自动检测缺失注释

### 使用 grep 检测

```bash
# 检测缺少文档注释的 pub fn
rg '^pub fn' -A 1 --type rust | rg '^[0-9]+:pub fn' -A 1 | rg -v '^[0-9]+-///'

# 检测缺少文档注释的 pub struct
rg '^pub struct' -A 1 --type rust | rg '^[0-9]+:pub struct' -A 1 | rg -v '^[0-9]+-///'
```

### 使用 cargo missing_docs

```bash
# 安装
cargo install cargo-missing-docs

# 运行检查
cargo missing-docs
```

### CI 配置

```yaml
# .github/workflows/docs.yml
- name: Check documentation
  run: |
    cargo doc --no-deps
    cargo test --doc
    cargo clippy -- -W clippy::missing_docs_in_private_items
```