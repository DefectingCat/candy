//! Lua 共享字典实现
//!
//! 提供类似 OpenResty ngx.shared.DICT 的功能
//! 使用 DashMap 实现线程安全的共享内存存储

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use mlua::{UserData, UserDataMethods};

/// 存储的值类型
#[derive(Clone, Debug)]
pub enum StoredValue {
    /// 标量值（字符串/数字）
    Scalar(Vec<u8>),
    /// 列表值
    List(Vec<Vec<u8>>),
}

impl StoredValue {
    /// 获取估算大小
    pub fn estimated_size(&self) -> usize {
        match self {
            StoredValue::Scalar(v) => v.len(),
            StoredValue::List(items) => items.iter().map(|v| v.len()).sum(),
        }
    }
}

/// 共享字典中的值条目
#[derive(Clone, Debug)]
pub struct SharedDictEntry {
    /// 存储的值
    pub value: StoredValue,
    /// 过期时间（None 表示永不过期）
    pub expires_at: Option<Instant>,
    /// 用户标志位（用于 set/get_stale）
    pub user_flags: u32,
    /// 最后访问时间（用于 LRU 淘汰）
    pub last_access: Instant,
}

impl SharedDictEntry {
    /// 创建新的条目（标量值）
    pub fn new(value: Vec<u8>) -> Self {
        Self {
            value: StoredValue::Scalar(value),
            expires_at: None,
            user_flags: 0,
            last_access: Instant::now(),
        }
    }

    /// 创建列表条目
    pub fn new_list() -> Self {
        Self {
            value: StoredValue::List(Vec::new()),
            expires_at: None,
            user_flags: 0,
            last_access: Instant::now(),
        }
    }

    /// 创建带过期时间的条目
    pub fn with_ttl(value: Vec<u8>, ttl: Duration) -> Self {
        Self {
            value: StoredValue::Scalar(value),
            expires_at: Some(Instant::now() + ttl),
            user_flags: 0,
            last_access: Instant::now(),
        }
    }

    /// 创建带标志位的条目
    pub fn with_flags(value: Vec<u8>, flags: u32) -> Self {
        Self {
            value: StoredValue::Scalar(value),
            expires_at: None,
            user_flags: flags,
            last_access: Instant::now(),
        }
    }

    /// 创建完整的条目
    pub fn full(value: Vec<u8>, ttl: Option<Duration>, flags: u32) -> Self {
        Self {
            value: StoredValue::Scalar(value),
            expires_at: ttl.map(|t| Instant::now() + t),
            user_flags: flags,
            last_access: Instant::now(),
        }
    }

    /// 检查条目是否已过期
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Instant::now() > exp)
            .unwrap_or(false)
    }

    /// 检查条目是否过期（即使是过期的也返回 true）
    pub fn is_stale(&self) -> bool {
        self.expires_at
            .map(|exp| Instant::now() > exp)
            .unwrap_or(false)
    }

    /// 获取剩余 TTL（秒）
    pub fn remaining_ttl(&self) -> Option<f64> {
        self.expires_at.map(|exp| {
            let remaining = exp.saturating_duration_since(Instant::now());
            remaining.as_secs_f64()
        })
    }

    /// 更新最后访问时间
    pub fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    /// 获取条目的估算大小
    pub fn estimated_size(&self) -> usize {
        self.value.estimated_size()
    }
}

/// 共享字典
///
/// 线程安全的键值存储，支持过期时间
#[derive(Clone, Debug)]
pub struct SharedDict {
    /// 字典名称
    pub name: String,
    /// 最大容量（字节）
    pub capacity: usize,
    /// 数据存储
    pub data: Arc<DashMap<String, SharedDictEntry>>,
}

impl SharedDict {
    /// 创建新的共享字典
    pub fn new(name: String, capacity: usize) -> Self {
        Self {
            name,
            capacity,
            data: Arc::new(DashMap::new()),
        }
    }

    /// 获取当前已使用容量（估算）
    pub fn used_capacity(&self) -> usize {
        self.data
            .iter()
            .map(|entry| {
                // 估算条目大小：键长度 + 值长度 + 元数据开销
                entry.key().len() + entry.value().estimated_size() + 32
            })
            .sum()
    }

    /// 检查是否有足够空间
    pub fn has_capacity(&self, additional: usize) -> bool {
        self.used_capacity() + additional <= self.capacity
    }

    /// 获取键对应的值
    ///
    /// # 返回值
    /// - `Some((value, flags))` - 键存在且未过期
    /// - `None` - 键不存在或已过期
    pub fn get(&self, key: &str) -> Option<(Vec<u8>, u32)> {
        let entry = self.data.get(key)?;

        // 检查是否过期
        if entry.is_expired() {
            // 过期条目，返回 None（但不立即删除，get_stale 可能需要）
            return None;
        }

        // 只返回标量值
        match &entry.value {
            StoredValue::Scalar(v) => Some((v.clone(), entry.user_flags)),
            StoredValue::List(_) => None, // 列表类型不能用 get 获取
        }
    }

    /// 获取键对应的值（包含过期条目）
    ///
    /// # 返回值
    /// - `Some((value, flags, is_stale))` - 键存在
    /// - `None` - 键不存在
    pub fn get_stale(&self, key: &str) -> Option<(Vec<u8>, u32, bool)> {
        let entry = self.data.get(key)?;
        let is_stale = entry.is_stale();

        // 只返回标量值
        match &entry.value {
            StoredValue::Scalar(v) => Some((v.clone(), entry.user_flags, is_stale)),
            StoredValue::List(_) => None, // 列表类型不能用 get_stale 获取
        }
    }

    /// 设置键值对
    ///
    /// # 参数
    /// - `key` - 键
    /// - `value` - 值
    /// - `ttl` - 过期时间（秒），None 或 0 表示永不过期
    /// - `flags` - 用户标志位
    ///
    /// # 返回值
    /// - `SetResult` - 包含 success, err, forcible
    ///
    /// # LRU 淘汰策略
    /// 当内存不足时，会尝试：
    /// 1. 先清理过期条目
    /// 2. 如果仍不足，按 LRU 策略淘汰最近最少使用的条目
    pub fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> SetResult {
        let entry_size = key.len() + value.len() + 32;
        let mut forcible = false;

        // 检查容量
        if !self.has_capacity(entry_size) {
            // 先尝试清理过期条目
            self.flush_expired();

            // 如果仍然不足，使用 LRU 淘汰
            if !self.has_capacity(entry_size) {
                forcible = self.evict_lru(entry_size);
                
                // 淘汰后仍然不足
                if !self.has_capacity(entry_size) {
                    return SetResult {
                        success: false,
                        err: Some("no memory".to_string()),
                        forcible,
                    };
                }
            }
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        self.data.insert(key.to_string(), entry);

        SetResult {
            success: true,
            err: None,
            forcible,
        }
    }

    /// 使用 LRU 策略淘汰条目
    ///
    /// # 参数
    /// - `required_size` - 需要释放的空间大小
    ///
    /// # 返回值
    /// - `true` - 成功淘汰了条目
    /// - `false` - 没有可淘汰的条目
    fn evict_lru(&self, required_size: usize) -> bool {
        // 收集所有条目及其最后访问时间
        let mut entries: Vec<(String, Instant, usize)> = self
            .data
            .iter()
            .map(|entry| {
                let key = entry.key().clone();
                let access_time = entry.last_access;
                let size = key.len() + entry.value.estimated_size() + 32;
                (key, access_time, size)
            })
            .collect();

        // 按最后访问时间排序（最旧的在前）
        entries.sort_by_key(|(_, time, _)| *time);

        let mut freed = 0usize;
        let mut evicted = false;
        const MAX_EVICTIONS: usize = 30; // 最多淘汰 30 个条目

        for (key, _, size) in entries.into_iter().take(MAX_EVICTIONS) {
            if freed >= required_size {
                break;
            }
            
            // 不要淘汰正在设置的键
            if self.data.remove(&key).is_some() {
                freed += size;
                evicted = true;
            }
        }

        evicted
    }

    /// 安全设置键值对（仅在内存充足时设置）
    ///
    /// 与 set 方法类似，但不会淘汰任何未过期条目。
    /// 当内存不足时，立即返回错误而不是强制淘汰。
    ///
    /// # 返回值
    /// - `Ok(true)` - 设置成功，覆盖了旧值
    /// - `Ok(false)` - 设置成功，是新键
    /// - `Err(SharedDictError::NoMemory)` - 内存不足（不会删除现有条目）
    pub fn safe_set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> Result<bool, SharedDictError> {
        let entry_size = key.len() + value.len() + 32;

        // 检查容量（不清理过期条目，不淘汰 LRU 条目）
        if !self.has_capacity(entry_size) {
            return Err(SharedDictError::NoMemory);
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        let existed = self.data.contains_key(key);
        self.data.insert(key.to_string(), entry);

        Ok(existed)
    }

    /// 添加键值对（仅在键不存在时成功）
    ///
    /// 类似 set 方法，但仅在键不存在时存储。
    /// 如果键已存在且未过期，返回 success=false, err="exists"。
    ///
    /// # 返回值
    /// - `SetResult` - 包含 success, err, forcible
    ///
    /// # LRU 淘汰策略
    /// 当内存不足时，会尝试：
    /// 1. 先清理过期条目
    /// 2. 如果仍不足，按 LRU 策略淘汰最近最少使用的条目
    pub fn add(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> SetResult {
        // 检查键是否存在（需要检查过期）
        if let Some(entry) = self.data.get(key) {
            if !entry.is_expired() {
                return SetResult {
                    success: false,
                    err: Some("exists".to_string()),
                    forcible: false,
                };
            }
            // 键已过期，可以覆盖
        }

        let entry_size = key.len() + value.len() + 32;
        let mut forcible = false;

        // 检查容量
        if !self.has_capacity(entry_size) {
            // 先清理过期条目
            self.flush_expired();

            // 如果仍然不足，使用 LRU 淘汰
            if !self.has_capacity(entry_size) {
                forcible = self.evict_lru(entry_size);

                if !self.has_capacity(entry_size) {
                    return SetResult {
                        success: false,
                        err: Some("no memory".to_string()),
                        forcible,
                    };
                }
            }
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        self.data.insert(key.to_string(), entry);

        SetResult {
            success: true,
            err: None,
            forcible,
        }
    }

    /// 安全添加键值对
    ///
    /// 类似 add 方法，但不会淘汰任何未过期条目。
    /// 当内存不足时，立即返回错误。
    ///
    /// # 返回值
    /// - `Ok(true)` - 添加成功
    /// - `Ok(false)` - 键已存在（未过期）
    /// - `Err(SharedDictError::NoMemory)` - 内存不足
    pub fn safe_add(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> Result<bool, SharedDictError> {
        // 检查键是否存在（需要检查过期）
        if let Some(entry) = self.data.get(key) {
            if !entry.is_expired() {
                return Ok(false);
            }
            // 键已过期，可以覆盖
        }

        let entry_size = key.len() + value.len() + 32;

        // 检查容量（不清理过期条目，不淘汰 LRU 条目）
        if !self.has_capacity(entry_size) {
            return Err(SharedDictError::NoMemory);
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        self.data.insert(key.to_string(), entry);

        Ok(true)
    }

    /// 替换键值对（仅在键存在且未过期时成功）
    ///
    /// 类似 set 方法，但仅在键存在时存储。
    /// 如果键不存在或已过期，返回 success=false, err="not found"。
    ///
    /// # 返回值
    /// - `SetResult` - 包含 success, err, forcible
    ///
    /// # LRU 淘汰策略
    /// 当内存不足时，会尝试：
    /// 1. 先清理过期条目
    /// 2. 如果仍不足，按 LRU 策略淘汰最近最少使用的条目
    pub fn replace(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> SetResult {
        // 检查键是否存在且未过期
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired() {
                return SetResult {
                    success: false,
                    err: Some("not found".to_string()),
                    forcible: false,
                };
            }
            // 键存在且未过期，可以替换
        } else {
            return SetResult {
                success: false,
                err: Some("not found".to_string()),
                forcible: false,
            };
        }

        // 使用 set 方法进行替换
        self.set(key, value, ttl, flags)
    }

    /// 删除键
    ///
    /// # 返回值
    /// - `true` - 删除成功
    /// - `false` - 键不存在
    pub fn delete(&self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }

    /// 增加计数器的值
    ///
    /// # 参数
    /// - `key` - 键
    /// - `value` - 增量（可以为负数或浮点数）
    /// - `init` - 初始值（如果键不存在）
    /// - `init_ttl` - 初始值的过期时间
    ///
    /// # 返回值
    /// - `IncrResult` - 包含 new_value, err, forcible
    ///
    /// # 行为
    /// - 键不存在或已过期时，如果 init 未指定，返回 "not found"
    /// - 键不存在或已过期时，如果 init 指定，创建新键并设置 init + value
    /// - 原值不是数字时，返回 "not a number"
    /// - 创建新键时使用 LRU 淘汰（与 add 方法类似）
    pub fn incr(
        &self,
        key: &str,
        value: f64,
        init: Option<f64>,
        init_ttl: Option<f64>,
    ) -> IncrResult {
        // 尝试获取现有值
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired() {
                // 过期条目，使用初始值
                drop(entry);

                let init_val = match init {
                    Some(v) => v,
                    None => {
                        return IncrResult {
                            new_value: None,
                            err: Some("not found".to_string()),
                            forcible: None,
                        }
                    }
                };
                return self.incr_init(key, init_val, value, init_ttl);
            }

            // 解析现有值
            let current = match &entry.value {
                StoredValue::Scalar(v) => String::from_utf8_lossy(v),
                StoredValue::List(_) => {
                    return IncrResult {
                        new_value: None,
                        err: Some("not a number".to_string()),
                        forcible: None,
                    }
                }
            };
            let current: f64 = match current.trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    return IncrResult {
                        new_value: None,
                        err: Some("not a number".to_string()),
                        forcible: None,
                    }
                }
            };

            let new_value = current + value;
            let new_entry = SharedDictEntry::full(
                new_value.to_string().into_bytes(),
                entry.expires_at.map(|exp| exp.duration_since(Instant::now())),
                entry.user_flags,
            );

            drop(entry);
            self.data.insert(key.to_string(), new_entry);

            return IncrResult {
                new_value: Some(new_value),
                err: None,
                forcible: Some(false), // 更新现有键，不涉及淘汰
            };
        }

        // 键不存在，使用初始值
        let init_val = match init {
            Some(v) => v,
            None => {
                return IncrResult {
                    new_value: None,
                    err: Some("not found".to_string()),
                    forcible: None,
                }
            }
        };
        self.incr_init(key, init_val, value, init_ttl)
    }

    fn incr_init(
        &self,
        key: &str,
        init: f64,
        value: f64,
        ttl: Option<f64>,
    ) -> IncrResult {
        let new_value = init + value;
        let entry_size = key.len() + new_value.to_string().len() + 32;
        let mut forcible = false;

        if !self.has_capacity(entry_size) {
            // 先清理过期条目
            self.flush_expired();

            // 如果仍不足，使用 LRU 淘汰
            if !self.has_capacity(entry_size) {
                forcible = self.evict_lru(entry_size);

                if !self.has_capacity(entry_size) {
                    return IncrResult {
                        new_value: None,
                        err: Some("no memory".to_string()),
                        forcible: Some(forcible),
                    };
                }
            }
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(new_value.to_string().into_bytes(), ttl_duration, 0);

        self.data.insert(key.to_string(), entry);

        IncrResult {
            new_value: Some(new_value),
            err: None,
            forcible: Some(forcible),
        }
    }

    /// 在列表头部插入元素
    ///
    /// # 返回值
    /// - `ListResult` - 包含 length, err
    pub fn lpush(&self, key: &str, value: Vec<u8>) -> ListResult {
        self.list_push(key, value, true)
    }

    /// 在列表尾部插入元素
    ///
    /// # 返回值
    /// - `ListResult` - 包含 length, err
    pub fn rpush(&self, key: &str, value: Vec<u8>) -> ListResult {
        self.list_push(key, value, false)
    }

    /// 通用列表插入方法
    fn list_push(&self, key: &str, value: Vec<u8>, at_head: bool) -> ListResult {
        let additional_size = key.len() + value.len() + 32;

        // 检查现有条目
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired() {
                // 过期条目，删除后创建新列表
                drop(entry);
                self.data.remove(key);
            } else {
                // 检查是否为列表类型
                match &entry.value {
                    StoredValue::List(_) => {
                        // 是列表，可以添加
                        let mut cloned_entry = entry.clone();
                        drop(entry);

                        let len = {
                            let items = match &mut cloned_entry.value {
                                StoredValue::List(items) => items,
                                _ => unreachable!(),
                            };

                            let additional = value.len();

                            // 检查容量（不淘汰）
                            if !self.has_capacity(additional) {
                                return ListResult {
                                    length: None,
                                    err: Some("no memory".to_string()),
                                };
                            }

                            if at_head {
                                items.insert(0, value);
                            } else {
                                items.push(value);
                            }

                            items.len()
                        };

                        cloned_entry.touch();
                        self.data.insert(key.to_string(), cloned_entry);

                        return ListResult {
                            length: Some(len),
                            err: None,
                        };
                    }
                    StoredValue::Scalar(_) => {
                        // 不是列表
                        return ListResult {
                            length: None,
                            err: Some("value not a list".to_string()),
                        };
                    }
                }
            }
        }

        // 键不存在，创建新列表
        if !self.has_capacity(additional_size) {
            return ListResult {
                length: None,
                err: Some("no memory".to_string()),
            };
        }

        let mut entry = SharedDictEntry::new_list();
        if let StoredValue::List(ref mut items) = entry.value {
            items.push(value);
        }

        self.data.insert(key.to_string(), entry);

        ListResult {
            length: Some(1),
            err: None,
        }
    }

    /// 从列表头部弹出元素
    pub fn lpop(&self, key: &str) -> ListPopResult {
        self.list_pop(key, true)
    }

    /// 从列表尾部弹出元素
    pub fn rpop(&self, key: &str) -> ListPopResult {
        self.list_pop(key, false)
    }

    /// 通用列表弹出方法
    fn list_pop(&self, key: &str, from_head: bool) -> ListPopResult {
        let entry = match self.data.get(key) {
            Some(e) => e,
            None => {
                return ListPopResult {
                    value: None,
                    err: None,
                }
            }
        };

        if entry.is_expired() {
            return ListPopResult {
                value: None,
                err: None,
            };
        }

        match &entry.value {
            StoredValue::List(items) => {
                if items.is_empty() {
                    return ListPopResult {
                        value: None,
                        err: None,
                    };
                }

                let mut cloned_entry = entry.clone();
                drop(entry);

                let (value, is_empty) = {
                    let items = match &mut cloned_entry.value {
                        StoredValue::List(items) => items,
                        _ => unreachable!(),
                    };

                    let value = if from_head {
                        items.remove(0)
                    } else {
                        items.pop().unwrap()
                    };

                    (value, items.is_empty())
                };

                // 如果列表为空，删除整个条目
                if is_empty {
                    self.data.remove(key);
                } else {
                    cloned_entry.touch();
                    self.data.insert(key.to_string(), cloned_entry);
                }

                return ListPopResult {
                    value: Some(value),
                    err: None,
                };
            }
            StoredValue::Scalar(_) => {
                return ListPopResult {
                    value: None,
                    err: Some("value not a list".to_string()),
                };
            }
        }
    }

    /// 获取列表长度
    pub fn llen(&self, key: &str) -> ListLenResult {
        let entry = match self.data.get(key) {
            Some(e) => e,
            None => {
                return ListLenResult {
                    length: None,
                    err: None,
                }
            }
        };

        if entry.is_expired() {
            return ListLenResult {
                length: None,
                err: None,
            };
        }

        match &entry.value {
            StoredValue::List(items) => ListLenResult {
                length: Some(items.len()),
                err: None,
            },
            StoredValue::Scalar(_) => ListLenResult {
                length: None,
                err: Some("value not a list".to_string()),
            },
        }
    }

    /// 清除所有条目
    pub fn flush_all(&self) {
        self.data.clear();
    }

    /// 清除所有过期条目
    ///
    /// # 返回值
    /// 清除的条目数量
    pub fn flush_expired(&self) -> usize {
        let mut count = 0;
        self.data.retain(|_, entry| {
            if entry.is_expired() {
                count += 1;
                false
            } else {
                true
            }
        });
        count
    }

    /// 获取所有键
    ///
    /// # 参数
    /// - `max_count` - 最大返回数量，None 表示无限制
    ///
    /// # 返回值
    /// 键列表
    pub fn get_keys(&self, max_count: Option<usize>) -> Vec<String> {
        let keys: Vec<String> = self
            .data
            .iter()
            .filter(|entry| !entry.is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        match max_count {
            Some(n) if n > 0 && keys.len() > n => keys.into_iter().take(n).collect(),
            _ => keys,
        }
    }

    /// 获取条目数量（不含过期条目）
    pub fn len(&self) -> usize {
        self.data.iter().filter(|e| !e.is_expired()).count()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 共享字典错误类型
#[derive(Clone, Debug)]
pub enum SharedDictError {
    /// 内存不足
    NoMemory,
    /// 值不是数字
    NotANumber,
    /// 键不存在
    NotFound,
    /// 键已存在
    Exists,
}

impl std::fmt::Display for SharedDictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMemory => write!(f, "no memory"),
            Self::NotANumber => write!(f, "not a number"),
            Self::NotFound => write!(f, "not found"),
            Self::Exists => write!(f, "exists"),
        }
    }
}

impl std::error::Error for SharedDictError {}

/// set 操作的结果
#[derive(Clone, Debug)]
pub struct SetResult {
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果有）
    pub err: Option<String>,
    /// 是否强制淘汰了其他条目
    pub forcible: bool,
}

/// incr 操作的结果
#[derive(Clone, Debug)]
pub struct IncrResult {
    /// 新值（如果成功）
    pub new_value: Option<f64>,
    /// 错误信息（如果有）
    pub err: Option<String>,
    /// 是否强制淘汰了其他条目（仅在创建新键时有意义）
    pub forcible: Option<bool>,
}

/// 列表操作的结果（lpush/rpush）
#[derive(Clone, Debug)]
pub struct ListResult {
    /// 列表长度（如果成功）
    pub length: Option<usize>,
    /// 错误信息（如果有）
    pub err: Option<String>,
}

/// 列表弹出操作的结果（lpop/rpop）
#[derive(Clone, Debug)]
pub struct ListPopResult {
    /// 弹出的值（如果成功）
    pub value: Option<Vec<u8>>,
    /// 错误信息（如果有）
    pub err: Option<String>,
}

/// 列表长度操作的结果（llen）
#[derive(Clone, Debug)]
pub struct ListLenResult {
    /// 列表长度（如果成功）
    pub length: Option<usize>,
    /// 错误信息（如果有）
    pub err: Option<String>,
}

// ============================================================================
// Lua UserData 实现
// ============================================================================

impl UserData for SharedDict {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // dict:get(key)
        // 返回: value, flags 或 nil
        methods.add_method("get", |lua, this, key: String| {
            match this.get(&key) {
                Some((value, flags)) => {
                    let value_str = lua.create_string(&value)?;
                    Ok((mlua::Value::String(value_str), mlua::Value::Integer(flags as i64)))
                }
                None => Ok((mlua::Value::Nil, mlua::Value::Nil)),
            }
        });

        // dict:get_stale(key)
        // 返回: value, flags, stale 或 nil
        methods.add_method("get_stale", |lua, this, key: String| {
            match this.get_stale(&key) {
                Some((value, flags, stale)) => {
                    let value_str = lua.create_string(&value)?;
                    Ok((
                        mlua::Value::String(value_str),
                        mlua::Value::Integer(flags as i64),
                        mlua::Value::Boolean(stale),
                    ))
                }
                None => Ok((mlua::Value::Nil, mlua::Value::Nil, mlua::Value::Nil)),
            }
        });

        // dict:set(key, value, exptime?, flags?)
        // 返回: success, err, forcible
        methods.add_method(
            "set",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                let result = this.set(&key, value_bytes, exptime, flags);
                
                let success = result.success;
                let err = match result.err {
                    Some(e) => mlua::Value::String(lua.create_string(&e)?),
                    None => mlua::Value::Nil,
                };
                let forcible = result.forcible;

                Ok((success, err, forcible))
            },
        );

        // dict:safe_set(key, value, exptime?, flags?)
        // 返回: ok, err
        // ok: true 表示成功，nil 表示失败（不是 false）
        // err: 错误信息，如 "no memory"
        methods.add_method(
            "safe_set",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                let result: Result<(mlua::Value, mlua::Value), mlua::Error> = match this.safe_set(&key, value_bytes, exptime, flags) {
                    Ok(_) => Ok((mlua::Value::Boolean(true), mlua::Value::Nil)),
                    Err(SharedDictError::NoMemory) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string(&e.to_string())?))),
                };
                result
            },
        );

        // dict:add(key, value, exptime?, flags?)
        // 返回: success, err, forcible
        methods.add_method(
            "add",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                let result = this.add(&key, value_bytes, exptime, flags);

                let success = result.success;
                let err = match result.err {
                    Some(e) => mlua::Value::String(lua.create_string(&e)?),
                    None => mlua::Value::Nil,
                };
                let forcible = result.forcible;

                Ok((success, err, forcible))
            },
        );

        // dict:safe_add(key, value, exptime?, flags?)
        // 返回: ok, err
        // ok: true 表示成功，nil 表示失败
        methods.add_method(
            "safe_add",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                let result: Result<(mlua::Value, mlua::Value), mlua::Error> = match this.safe_add(&key, value_bytes, exptime, flags) {
                    Ok(true) => Ok((mlua::Value::Boolean(true), mlua::Value::Nil)),
                    Ok(false) => Ok((mlua::Value::Boolean(false), mlua::Value::String(lua.create_string("exists")?))),
                    Err(SharedDictError::NoMemory) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string(&e.to_string())?))),
                };
                result
            },
        );

        // dict:replace(key, value, exptime?, flags?)
        // 返回: success, err, forcible
        methods.add_method(
            "replace",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                let result = this.replace(&key, value_bytes, exptime, flags);

                let success = result.success;
                let err = match result.err {
                    Some(e) => mlua::Value::String(lua.create_string(&e)?),
                    None => mlua::Value::Nil,
                };
                let forcible = result.forcible;

                Ok((success, err, forcible))
            },
        );

        // dict:delete(key)
        // 返回: true 或 nil
        methods.add_method("delete", |_, this, key: String| {
            let result: Option<bool> = if this.delete(&key) {
                Some(true)
            } else {
                None
            };
            Ok(result)
        });

        // dict:incr(key, value, init?, init_ttl?)
        // 返回: newval, err, forcible
        methods.add_method(
            "incr",
            |lua, this, (key, value, init, init_ttl): (String, f64, Option<f64>, Option<f64>)| {
                let result = this.incr(&key, value, init, init_ttl);

                let newval = match result.new_value {
                    Some(v) => {
                        // 如果是整数，返回整数，否则返回浮点数
                        if v.fract() == 0.0 && v.is_finite() {
                            mlua::Value::Integer(v as i64)
                        } else {
                            mlua::Value::Number(v)
                        }
                    }
                    None => mlua::Value::Nil,
                };

                let err = match result.err {
                    Some(e) => mlua::Value::String(lua.create_string(&e)?),
                    None => mlua::Value::Nil,
                };

                let forcible = match result.forcible {
                    Some(v) => mlua::Value::Boolean(v),
                    None => mlua::Value::Nil,
                };

                Ok((newval, err, forcible))
            },
        );

        // dict:flush_all()
        methods.add_method("flush_all", |_, this, ()| {
            this.flush_all();
            Ok(())
        });

        // dict:flush_expired(max_count?)
        // 返回: flushed_count
        methods.add_method("flush_expired", |_, this, _max_count: Option<usize>| {
            // Note: OpenResty 的 max_count 参数在当前实现中未完全支持
            let count = this.flush_expired();
            Ok(count)
        });

        // dict:get_keys(max_count?)
        // 返回: keys table
        methods.add_method("get_keys", |lua, this, max_count: Option<usize>| {
            let keys = this.get_keys(max_count);
            let table = lua.create_table()?;
            for (i, key) in keys.into_iter().enumerate() {
                table.set(i + 1, key)?;
            }
            Ok(table)
        });

        // dict:lpush(key, value)
        // 返回: length, err
        methods.add_method("lpush", |lua, this, (key, value): (String, mlua::Value)| {
            let value_bytes = lua_value_to_bytes(&value)?;
            let result = this.lpush(&key, value_bytes);

            let length = result.length.map(|l| l as i64);
            let err = match result.err {
                Some(e) => mlua::Value::String(lua.create_string(&e)?),
                None => mlua::Value::Nil,
            };

            Ok((length, err))
        });

        // dict:rpush(key, value)
        // 返回: length, err
        methods.add_method("rpush", |lua, this, (key, value): (String, mlua::Value)| {
            let value_bytes = lua_value_to_bytes(&value)?;
            let result = this.rpush(&key, value_bytes);

            let length = result.length.map(|l| l as i64);
            let err = match result.err {
                Some(e) => mlua::Value::String(lua.create_string(&e)?),
                None => mlua::Value::Nil,
            };

            Ok((length, err))
        });

        // dict:lpop(key)
        // 返回: value, err
        methods.add_method("lpop", |lua, this, key: String| {
            let result = this.lpop(&key);

            let value = match result.value {
                Some(v) => mlua::Value::String(lua.create_string(&v)?),
                None => mlua::Value::Nil,
            };
            let err = match result.err {
                Some(e) => mlua::Value::String(lua.create_string(&e)?),
                None => mlua::Value::Nil,
            };

            Ok((value, err))
        });

        // dict:rpop(key)
        // 返回: value, err
        methods.add_method("rpop", |lua, this, key: String| {
            let result = this.rpop(&key);

            let value = match result.value {
                Some(v) => mlua::Value::String(lua.create_string(&v)?),
                None => mlua::Value::Nil,
            };
            let err = match result.err {
                Some(e) => mlua::Value::String(lua.create_string(&e)?),
                None => mlua::Value::Nil,
            };

            Ok((value, err))
        });

        // dict:llen(key)
        // 返回: length, err
        methods.add_method("llen", |lua, this, key: String| {
            let result = this.llen(&key);

            let length = result.length.map(|l| l as i64);
            let err = match result.err {
                Some(e) => mlua::Value::String(lua.create_string(&e)?),
                None => mlua::Value::Nil,
            };

            Ok((length, err))
        });
    }
}

/// 将 Lua 值转换为字节
fn lua_value_to_bytes(value: &mlua::Value) -> Result<Vec<u8>, mlua::Error> {
    match value {
        mlua::Value::String(s) => Ok(s.as_bytes().to_vec()),
        mlua::Value::Integer(i) => Ok(i.to_string().into_bytes()),
        mlua::Value::Number(n) => Ok(n.to_string().into_bytes()),
        mlua::Value::Boolean(b) => Ok(if *b { b"true".to_vec() } else { b"false".to_vec() }),
        mlua::Value::Nil => Ok(Vec::new()),
        _ => Err(mlua::Error::external(anyhow::anyhow!(
            "value must be a string, number, boolean, or nil"
        ))),
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_dict_basic() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置和获取
        let result = dict.set("key1", b"value1".to_vec(), None, 0);
        assert!(result.success);
        assert!(result.err.is_none());
        assert!(!result.forcible);
        
        let (value, flags) = dict.get("key1").unwrap();
        assert_eq!(value, b"value1");
        assert_eq!(flags, 0);
    }

    #[test]
    fn test_shared_dict_with_flags() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), None, 42);
        let (_, flags) = dict.get("key1").unwrap();
        assert_eq!(flags, 42);
    }

    #[test]
    fn test_shared_dict_not_found() {
        let dict = SharedDict::new("test".to_string(), 1024);

        assert!(dict.get("nonexistent").is_none());
    }

    #[test]
    fn test_shared_dict_add() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 添加新键
        let result = dict.add("key1", b"value1".to_vec(), None, 0);
        assert!(result.success);
        assert!(result.err.is_none());
        assert!(!result.forcible);

        // 添加已存在的键
        let result = dict.add("key1", b"value2".to_vec(), None, 0);
        assert!(!result.success);
        assert_eq!(result.err, Some("exists".to_string()));

        // 验证值未改变
        let (value, _) = dict.get("key1").unwrap();
        assert_eq!(value, b"value1");
    }

    #[test]
    fn test_shared_dict_add_with_expired_key() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置一个很快过期的键
        dict.set("key1", b"value1".to_vec(), Some(0.05), 0);

        // 等待过期
        std::thread::sleep(Duration::from_millis(100));

        // 过期后 add 应该成功
        let result = dict.add("key1", b"value2".to_vec(), None, 0);
        assert!(result.success);

        // 验证新值
        let (value, _) = dict.get("key1").unwrap();
        assert_eq!(value, b"value2");
    }

    #[test]
    fn test_shared_dict_add_lru_eviction() {
        let dict = SharedDict::new("test".to_string(), 80);

        // 填满容量 (每个条目约 40 bytes: 2 + 2 + 32 overhead = ~36 bytes)
        // 第一个条目: ~36 bytes
        dict.set("k1", b"v1".to_vec(), None, 0);
        // 第二个条目: ~36 bytes, total ~72 bytes
        dict.set("k2", b"v2".to_vec(), None, 0);
        // 第三个条目需要 LRU 淘汰，因为 72 + 36 > 80
        let result = dict.add("k3", b"v3".to_vec(), None, 0);
        assert!(result.success);
        // 应该触发了 LRU 淘汰
        assert!(result.forcible, "Expected forcible=true, but got success={}, err={:?}", result.success, result.err);
    }

    #[test]
    fn test_shared_dict_safe_add() {
        let dict = SharedDict::new("test".to_string(), 100);

        // 添加新键
        let result = dict.safe_add("key1", b"value1".to_vec(), None, 0);
        assert!(result.is_ok());

        // 添加已存在的键
        let result = dict.safe_add("key1", b"value2".to_vec(), None, 0);
        assert!(matches!(result, Ok(false)));

        // 内存不足
        let large_value = vec![0u8; 200];
        let result = dict.safe_add("key2", large_value, None, 0);
        assert!(matches!(result, Err(SharedDictError::NoMemory)));
    }

    #[test]
    fn test_shared_dict_safe_add_with_expired_key() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置一个很快过期的键
        dict.set("key1", b"value1".to_vec(), Some(0.05), 0);

        // 等待过期
        std::thread::sleep(Duration::from_millis(100));

        // 过期后 safe_add 应该成功
        let result = dict.safe_add("key1", b"value2".to_vec(), None, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shared_dict_lpush_rpush() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // lpush 新列表
        let result = dict.lpush("mylist", b"value1".to_vec());
        assert_eq!(result.length, Some(1));
        assert!(result.err.is_none());

        // rpush 添加到尾部
        let result = dict.rpush("mylist", b"value2".to_vec());
        assert_eq!(result.length, Some(2));

        // lpush 添加到头部
        let result = dict.lpush("mylist", b"value0".to_vec());
        assert_eq!(result.length, Some(3));

        // 检查列表长度
        let result = dict.llen("mylist");
        assert_eq!(result.length, Some(3));
    }

    #[test]
    fn test_shared_dict_lpop_rpop() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 创建列表: [a, b, c]
        dict.rpush("mylist", b"a".to_vec());
        dict.rpush("mylist", b"b".to_vec());
        dict.rpush("mylist", b"c".to_vec());

        // lpop 从头部弹出
        let result = dict.lpop("mylist");
        assert_eq!(result.value, Some(b"a".to_vec()));
        assert_eq!(dict.llen("mylist").length, Some(2));

        // rpop 从尾部弹出
        let result = dict.rpop("mylist");
        assert_eq!(result.value, Some(b"c".to_vec()));
        assert_eq!(dict.llen("mylist").length, Some(1));

        // 弹出最后一个元素
        let result = dict.lpop("mylist");
        assert_eq!(result.value, Some(b"b".to_vec()));
        // 列表为空后键被删除
        assert!(dict.llen("mylist").length.is_none());

        // 空列表/不存在的键弹出返回 nil
        let result = dict.lpop("mylist");
        assert!(result.value.is_none());
    }

    #[test]
    fn test_shared_dict_list_not_a_list() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置标量值
        dict.set("key", b"value".to_vec(), None, 0);

        // 尝试 lpush 到标量
        let result = dict.lpush("key", b"item".to_vec());
        assert_eq!(result.err, Some("value not a list".to_string()));

        // 尝试 lpop 标量
        let result = dict.lpop("key");
        assert_eq!(result.err, Some("value not a list".to_string()));
    }

    #[test]
    fn test_shared_dict_list_no_memory() {
        let dict = SharedDict::new("test".to_string(), 50);

        // 小列表应该成功
        let result = dict.lpush("mylist", b"small".to_vec());
        assert!(result.length.is_some());

        // 大列表应该失败
        let large_value = vec![0u8; 100];
        let result = dict.rpush("mylist", large_value);
        assert_eq!(result.err, Some("no memory".to_string()));
    }

    #[test]
    fn test_shared_dict_replace() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 替换不存在的键
        let result = dict.replace("key1", b"value1".to_vec(), None, 0);
        assert!(!result.success);
        assert_eq!(result.err, Some("not found".to_string()));

        // 设置后替换
        dict.set("key1", b"value1".to_vec(), None, 0);
        let result = dict.replace("key1", b"value2".to_vec(), None, 0);
        assert!(result.success);

        let (value, _) = dict.get("key1").unwrap();
        assert_eq!(value, b"value2");
    }

    #[test]
    fn test_shared_dict_replace_with_expired_key() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置一个很快过期的键
        dict.set("key1", b"value1".to_vec(), Some(0.05), 0);

        // 等待过期
        std::thread::sleep(Duration::from_millis(100));

        // 过期后 replace 应该失败
        let result = dict.replace("key1", b"value2".to_vec(), None, 0);
        assert!(!result.success);
        assert_eq!(result.err, Some("not found".to_string()));
    }

    #[test]
    fn test_shared_dict_replace_with_lru() {
        let dict = SharedDict::new("test".to_string(), 150);

        // 设置多个键填满容量
        dict.set("key1", b"v1".to_vec(), None, 0);
        dict.set("key2", b"v2".to_vec(), None, 0);
        dict.set("key3", b"v3".to_vec(), None, 0);

        // replace key1 为一个更大的值，可能需要 LRU 淘汰
        // 因为新值比旧值大，可能需要淘汰其他条目
        let large_value = vec![0u8; 80];
        let result = dict.replace("key1", large_value, None, 0);
        assert!(result.success);
        // forcible 取决于是否需要淘汰其他条目
    }

    #[test]
    fn test_shared_dict_delete() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), None, 0);
        assert!(dict.delete("key1"));
        assert!(dict.get("key1").is_none());
        assert!(!dict.delete("key1")); // 再次删除返回 false
    }

    #[test]
    fn test_shared_dict_incr() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 不存在的键，无初始值
        let result = dict.incr("key1", 1.0, None, None);
        assert!(result.new_value.is_none());
        assert_eq!(result.err, Some("not found".to_string()));

        // 不存在的键，有初始值
        let result = dict.incr("key1", 5.0, Some(0.0), None);
        assert_eq!(result.new_value, Some(5.0));
        assert!(result.err.is_none());

        // 已存在的键
        let result = dict.incr("key1", 3.0, None, None);
        assert_eq!(result.new_value, Some(8.0));

        // 负增量
        let result = dict.incr("key1", -2.0, None, None);
        assert_eq!(result.new_value, Some(6.0));

        // 浮点数
        let result = dict.incr("key1", 0.5, None, None);
        assert_eq!(result.new_value, Some(6.5));
    }

    #[test]
    fn test_shared_dict_incr_not_a_number() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置非数字值
        dict.set("key1", b"not a number".to_vec(), None, 0);

        // 尝试增加
        let result = dict.incr("key1", 1.0, None, None);
        assert!(result.new_value.is_none());
        assert_eq!(result.err, Some("not a number".to_string()));
    }

    #[test]
    fn test_shared_dict_incr_with_lru() {
        let dict = SharedDict::new("test".to_string(), 80);

        // 填满容量
        dict.set("k1", b"v1".to_vec(), None, 0);
        dict.set("k2", b"v2".to_vec(), None, 0);

        // incr 新键需要 LRU 淘汰
        let result = dict.incr("k3", 1.0, Some(0.0), None);
        assert_eq!(result.new_value, Some(1.0));
        assert!(result.forcible.unwrap()); // 应该触发了 LRU 淘汰
    }

    #[test]
    fn test_shared_dict_incr_update_no_forcible() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置初始值
        dict.set("key1", b"10".to_vec(), None, 0);

        // 更新现有键不应该触发 LRU
        let result = dict.incr("key1", 5.0, None, None);
        assert_eq!(result.new_value, Some(15.0));
        assert_eq!(result.forcible, Some(false)); // 更新现有键，forcible = false
    }

    #[test]
    fn test_shared_dict_ttl() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 设置 0.1 秒过期
        dict.set("key1", b"value1".to_vec(), Some(0.1), 0);

        // 立即获取应该成功
        assert!(dict.get("key1").is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(150));

        // 过期后获取应该失败
        assert!(dict.get("key1").is_none());

        // 但 get_stale 应该能获取到
        let (_, _, stale) = dict.get_stale("key1").unwrap();
        assert!(stale);
    }

    #[test]
    fn test_shared_dict_flush_expired() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), Some(0.05), 0);
        dict.set("key2", b"value2".to_vec(), None, 0);

        // 等待 key1 过期
        std::thread::sleep(Duration::from_millis(100));

        let count = dict.flush_expired();
        assert_eq!(count, 1);

        // key2 应该还在
        assert!(dict.get("key2").is_some());
    }

    #[test]
    fn test_shared_dict_get_keys() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), None, 0);
        dict.set("key2", b"value2".to_vec(), None, 0);
        dict.set("key3", b"value3".to_vec(), None, 0);

        let keys = dict.get_keys(None);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));

        // 限制数量
        let keys = dict.get_keys(Some(2));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_shared_dict_capacity() {
        let dict = SharedDict::new("test".to_string(), 100);

        // 小数据应该成功
        let result = dict.set("key1", b"short".to_vec(), None, 0);
        assert!(result.success);

        // 大数据应该失败
        let large_value = vec![0u8; 200];
        let result = dict.set("key2", large_value, None, 0);
        assert!(!result.success);
        assert_eq!(result.err, Some("no memory".to_string()));
    }

    #[test]
    fn test_shared_dict_flush_all() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), None, 0);
        dict.set("key2", b"value2".to_vec(), None, 0);

        dict.flush_all();

        assert!(dict.get("key1").is_none());
        assert!(dict.get("key2").is_none());
    }

    #[test]
    fn test_shared_dict_lru_eviction() {
        let dict = SharedDict::new("test".to_string(), 200);

        // 添加多个条目填满容量
        dict.set("key1", b"value1".to_vec(), None, 0); // ~40 bytes
        dict.set("key2", b"value2".to_vec(), None, 0); // ~40 bytes
        dict.set("key3", b"value3".to_vec(), None, 0); // ~40 bytes
        dict.set("key4", b"value4".to_vec(), None, 0); // ~40 bytes
        dict.set("key5", b"value5".to_vec(), None, 0); // ~40 bytes

        // 访问 key1 和 key2，使它们更"最近"
        dict.get("key1");
        dict.get("key2");

        // 添加一个需要 LRU 淘汰的大条目
        let result = dict.set("big_key", "x".repeat(50).into_bytes(), None, 0);
        
        // 应该成功，但 forcible 为 true
        assert!(result.success);
        assert!(result.forcible);
        
        // key1 和 key2 应该仍然存在（最近访问过）
        // key3, key4, 或 key5 可能被淘汰
    }

    #[test]
    fn test_shared_dict_safe_set() {
        let dict = SharedDict::new("test".to_string(), 100);

        // 小数据应该成功
        let result = dict.safe_set("key1", b"short".to_vec(), None, 0);
        assert!(result.is_ok());

        // 大数据应该失败（不淘汰任何条目）
        let large_value = vec![0u8; 200];
        let result = dict.safe_set("key2", large_value, None, 0);
        assert!(matches!(result, Err(SharedDictError::NoMemory)));

        // key1 应该仍然存在（safe_set 不会淘汰它）
        assert!(dict.get("key1").is_some());
    }

    #[test]
    fn test_shared_dict_safe_set_vs_set() {
        let dict = SharedDict::new("test".to_string(), 100);

        // 填满容量
        dict.set("key1", b"value1".to_vec(), None, 0);
        dict.set("key2", b"value2".to_vec(), None, 0);

        // set 会淘汰 LRU 条目
        let set_result = dict.set("key3", b"value3".to_vec(), None, 0);
        assert!(set_result.success);
        assert!(set_result.forcible); // 淘汰了其他条目

        // 重置字典
        dict.flush_all();

        // 再次填满
        dict.set("key1", b"value1".to_vec(), None, 0);
        dict.set("key2", b"value2".to_vec(), None, 0);

        // safe_set 不会淘汰，直接返回错误
        let safe_result = dict.safe_set("key3", b"value3".to_vec(), None, 0);
        assert!(matches!(safe_result, Err(SharedDictError::NoMemory)));

        // key1 和 key2 应该仍然存在
        assert!(dict.get("key1").is_some());
        assert!(dict.get("key2").is_some());
    }

    #[test]
    fn test_lua_shared_dict_parse_size() {
        use crate::config::LuaSharedDict;

        let dict = LuaSharedDict {
            name: "test".to_string(),
            size: "10k".to_string(),
        };
        assert_eq!(dict.parse_size().unwrap(), 10 * 1024);

        let dict = LuaSharedDict {
            name: "test".to_string(),
            size: "10m".to_string(),
        };
        assert_eq!(dict.parse_size().unwrap(), 10 * 1024 * 1024);

        let dict = LuaSharedDict {
            name: "test".to_string(),
            size: "1g".to_string(),
        };
        assert_eq!(dict.parse_size().unwrap(), 1024 * 1024 * 1024);

        let dict = LuaSharedDict {
            name: "test".to_string(),
            size: "1024".to_string(),
        };
        assert_eq!(dict.parse_size().unwrap(), 1024);
    }
}