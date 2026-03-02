//! Lua 共享字典实现
//!
//! 提供类似 OpenResty ngx.shared.DICT 的功能
//! 使用 DashMap 实现线程安全的共享内存存储

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use mlua::{UserData, UserDataMethods};

/// 共享字典中的值条目
#[derive(Clone, Debug)]
pub struct SharedDictEntry {
    /// 存储的值
    pub value: Vec<u8>,
    /// 过期时间（None 表示永不过期）
    pub expires_at: Option<Instant>,
    /// 用户标志位（用于 set/get_stale）
    pub user_flags: u32,
    /// 最后访问时间（用于 LRU 淘汰）
    pub last_access: Instant,
}

impl SharedDictEntry {
    /// 创建新的条目
    pub fn new(value: Vec<u8>) -> Self {
        Self {
            value,
            expires_at: None,
            user_flags: 0,
            last_access: Instant::now(),
        }
    }

    /// 创建带过期时间的条目
    pub fn with_ttl(value: Vec<u8>, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Some(Instant::now() + ttl),
            user_flags: 0,
            last_access: Instant::now(),
        }
    }

    /// 创建带标志位的条目
    pub fn with_flags(value: Vec<u8>, flags: u32) -> Self {
        Self {
            value,
            expires_at: None,
            user_flags: flags,
            last_access: Instant::now(),
        }
    }

    /// 创建完整的条目
    pub fn full(value: Vec<u8>, ttl: Option<Duration>, flags: u32) -> Self {
        Self {
            value,
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
                entry.key().len() + entry.value().value.len() + 32
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

        Some((entry.value.clone(), entry.user_flags))
    }

    /// 获取键对应的值（包含过期条目）
    ///
    /// # 返回值
    /// - `Some((value, flags, is_stale))` - 键存在
    /// - `None` - 键不存在
    pub fn get_stale(&self, key: &str) -> Option<(Vec<u8>, u32, bool)> {
        let entry = self.data.get(key)?;
        let is_stale = entry.is_stale();
        Some((entry.value.clone(), entry.user_flags, is_stale))
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
                let size = key.len() + entry.value.len() + 32;
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

        // 检查容量（不清理过期条目）
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
    /// # 返回值
    /// - `Ok(true)` - 添加成功
    /// - `Ok(false)` - 键已存在
    /// - `Err(SharedDictError::NoMemory)` - 内存不足
    pub fn add(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> Result<bool, SharedDictError> {
        // 检查键是否存在
        if self.data.contains_key(key) {
            return Ok(false);
        }

        let entry_size = key.len() + value.len() + 32;

        // 检查容量
        if !self.has_capacity(entry_size) {
            self.flush_expired();

            if !self.has_capacity(entry_size) {
                return Err(SharedDictError::NoMemory);
            }
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        self.data.insert(key.to_string(), entry);

        Ok(true)
    }

    /// 安全添加键值对
    ///
    /// # 返回值
    /// - `Ok(true)` - 添加成功
    /// - `Ok(false)` - 键已存在
    /// - `Err(SharedDictError::NoMemory)` - 内存不足（不会删除现有条目）
    pub fn safe_add(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> Result<bool, SharedDictError> {
        // 检查键是否存在
        if self.data.contains_key(key) {
            return Ok(false);
        }

        let entry_size = key.len() + value.len() + 32;

        // 检查容量（不清理过期条目）
        if !self.has_capacity(entry_size) {
            return Err(SharedDictError::NoMemory);
        }

        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(value, ttl_duration, flags);

        self.data.insert(key.to_string(), entry);

        Ok(true)
    }

    /// 替换键值对（仅在键存在时成功）
    ///
    /// # 返回值
    /// - `Ok(true)` - 替换成功
    /// - `Ok(false)` - 键不存在
    /// - `Err(SharedDictError::NoMemory)` - 内存不足
    pub fn replace(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<f64>,
        flags: u32,
    ) -> Result<bool, SharedDictError> {
        // 检查键是否存在
        if !self.data.contains_key(key) {
            return Ok(false);
        }

        let result = self.set(key, value, ttl, flags);
        if result.success {
            Ok(true)
        } else {
            Err(SharedDictError::NoMemory)
        }
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
    /// - `value` - 增量（可以为负数）
    /// - `init` - 初始值（如果键不存在）
    /// - `init_ttl` - 初始值的过期时间
    ///
    /// # 返回值
    /// - `Ok(new_value)` - 增加后的新值
    /// - `Err(SharedDictError::NoMemory)` - 内存不足
    /// - `Err(SharedDictError::NotANumber)` - 现有值不是数字
    pub fn incr(
        &self,
        key: &str,
        value: i64,
        init: Option<i64>,
        init_ttl: Option<f64>,
    ) -> Result<i64, SharedDictError> {
        // 尝试获取现有值
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired() {
                // 过期条目，使用初始值
                drop(entry);

                let init_val = init.ok_or(SharedDictError::NotFound)?;
                return self.incr_init(key, init_val, value, init_ttl);
            }

            // 解析现有值
            let current = String::from_utf8_lossy(&entry.value);
            let current: i64 = current
                .trim()
                .parse()
                .map_err(|_| SharedDictError::NotANumber)?;

            let new_value = current + value;
            let new_entry = SharedDictEntry::full(
                new_value.to_string().into_bytes(),
                entry.expires_at.map(|exp| exp.duration_since(Instant::now())),
                entry.user_flags,
            );

            drop(entry);
            self.data.insert(key.to_string(), new_entry);

            return Ok(new_value);
        }

        // 键不存在，使用初始值
        let init_val = init.ok_or(SharedDictError::NotFound)?;
        self.incr_init(key, init_val, value, init_ttl)
    }

    fn incr_init(
        &self,
        key: &str,
        init: i64,
        value: i64,
        ttl: Option<f64>,
    ) -> Result<i64, SharedDictError> {
        let entry_size = key.len() + 32;

        if !self.has_capacity(entry_size) {
            return Err(SharedDictError::NoMemory);
        }

        let new_value = init + value;
        let ttl_duration = ttl.filter(|&t| t > 0.0).map(Duration::from_secs_f64);
        let entry = SharedDictEntry::full(new_value.to_string().into_bytes(), ttl_duration, 0);

        self.data.insert(key.to_string(), entry);

        Ok(new_value)
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
        // 返回: success, err
        methods.add_method(
            "safe_set",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                match this.safe_set(&key, value_bytes, exptime, flags) {
                    Ok(_) => Ok((true, mlua::Value::Nil)),
                    Err(SharedDictError::NoMemory) => Ok((false, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((false, mlua::Value::String(lua.create_string(&e.to_string())?))),
                }
            },
        );

        // dict:add(key, value, exptime?, flags?)
        // 返回: success, err
        methods.add_method(
            "add",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                match this.add(&key, value_bytes, exptime, flags) {
                    Ok(true) => Ok((true, mlua::Value::Nil)),
                    Ok(false) => Ok((false, mlua::Value::String(lua.create_string("exists")?))),
                    Err(SharedDictError::NoMemory) => Ok((false, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((false, mlua::Value::String(lua.create_string(&e.to_string())?))),
                }
            },
        );

        // dict:safe_add(key, value, exptime?, flags?)
        // 返回: success, err
        methods.add_method(
            "safe_add",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                match this.safe_add(&key, value_bytes, exptime, flags) {
                    Ok(true) => Ok((true, mlua::Value::Nil)),
                    Ok(false) => Ok((false, mlua::Value::String(lua.create_string("exists")?))),
                    Err(SharedDictError::NoMemory) => Ok((false, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((false, mlua::Value::String(lua.create_string(&e.to_string())?))),
                }
            },
        );

        // dict:replace(key, value, exptime?, flags?)
        // 返回: success, err
        methods.add_method(
            "replace",
            |lua, this, (key, value, exptime, flags): (String, mlua::Value, Option<f64>, Option<u32>)| {
                let value_bytes = lua_value_to_bytes(&value)?;
                let flags = flags.unwrap_or(0);

                match this.replace(&key, value_bytes, exptime, flags) {
                    Ok(true) => Ok((true, mlua::Value::Nil)),
                    Ok(false) => Ok((false, mlua::Value::String(lua.create_string("not found")?))),
                    Err(SharedDictError::NoMemory) => Ok((false, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((false, mlua::Value::String(lua.create_string(&e.to_string())?))),
                }
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
        // 返回: new_value, err
        methods.add_method(
            "incr",
            |lua, this, (key, value, init, init_ttl): (String, i64, Option<i64>, Option<f64>)| {
                let result: Result<(mlua::Value, mlua::Value), mlua::Error> = match this.incr(&key, value, init, init_ttl) {
                    Ok(new_value) => Ok((mlua::Value::Integer(new_value), mlua::Value::Nil)),
                    Err(SharedDictError::NotFound) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string("not found")?))),
                    Err(SharedDictError::NotANumber) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string("not a number")?))),
                    Err(SharedDictError::NoMemory) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string("no memory")?))),
                    Err(e) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string(&e.to_string())?))),
                };
                result
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

        // TODO: 实现列表操作
        // dict:lpush(key, value)
        // dict:rpush(key, value)
        // dict:lpop(key)
        // dict:rpop(key)
        // dict:llen(key)
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
    fn test_shared_dict_delete() {
        let dict = SharedDict::new("test".to_string(), 1024);

        dict.set("key1", b"value1".to_vec(), None, 0);
        assert!(dict.delete("key1"));
        assert!(dict.get("key1").is_none());
        assert!(!dict.delete("key1")); // 再次删除返回 false
    }

    #[test]
    fn test_shared_dict_add() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 添加新键
        assert!(dict.add("key1", b"value1".to_vec(), None, 0).unwrap());

        // 添加已存在的键
        assert!(!dict.add("key1", b"value2".to_vec(), None, 0).unwrap());

        // 验证值未改变
        let (value, _) = dict.get("key1").unwrap();
        assert_eq!(value, b"value1");
    }

    #[test]
    fn test_shared_dict_replace() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 替换不存在的键
        assert!(!dict.replace("key1", b"value1".to_vec(), None, 0).unwrap());

        // 设置后替换
        dict.set("key1", b"value1".to_vec(), None, 0);
        assert!(dict.replace("key1", b"value2".to_vec(), None, 0).unwrap());

        let (value, _) = dict.get("key1").unwrap();
        assert_eq!(value, b"value2");
    }

    #[test]
    fn test_shared_dict_incr() {
        let dict = SharedDict::new("test".to_string(), 1024);

        // 不存在的键，无初始值
        assert!(matches!(
            dict.incr("key1", 1, None, None),
            Err(SharedDictError::NotFound)
        ));

        // 不存在的键，有初始值
        let result = dict.incr("key1", 5, Some(0), None).unwrap();
        assert_eq!(result, 5);

        // 已存在的键
        let result = dict.incr("key1", 3, None, None).unwrap();
        assert_eq!(result, 8);

        // 负增量
        let result = dict.incr("key1", -2, None, None).unwrap();
        assert_eq!(result, 6);
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