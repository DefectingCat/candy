use anyhow::anyhow;
use http::{HeaderName, HeaderValue};
use mlua::{UserData, UserDataMethods};

use super::{
    constants::*,
    structures::{CandyHeaders, CandyReq, CandyResp, RequestContext},
    utils::UriArgs,
};

impl UserData for CandyResp {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // get_headers(): 返回所有响应头的 table
        methods.add_method("get_headers", |lua, this, ()| {
            this.headers.get_headers_table(lua)
        });
    }
}

impl UserData for CandyReq {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // is_internal(): 返回是否为内部请求
        // 在 Candy 中，目前没有子请求机制，始终返回 false
        methods.add_method("is_internal", |_, this, ()| Ok(this.is_internal));

        // start_time(): 返回请求开始时间（秒，包含毫秒小数）
        methods.add_method("start_time", |lua, this, ()| lua.pack(this.start_time));

        // http_version(): 返回 HTTP 版本号
        methods.add_method("http_version", |lua, this, ()| match this.http_version {
            Some(v) => lua.pack(v),
            None => Ok(mlua::Value::Nil),
        });

        // raw_header(no_request_line?): 返回原始请求头
        // raw_header() - 包含请求行
        // raw_header(true) - 不包含请求行
        methods.add_method("raw_header", |lua, this, no_request_line: Option<bool>| {
            let skip_request_line = no_request_line.unwrap_or(false);
            if skip_request_line {
                lua.pack(this.raw_header.clone())
            } else {
                let full = format!("{}\r\n{}", this.request_line, this.raw_header);
                lua.pack(full)
            }
        });

        // get_method(): 返回请求方法名称
        methods.add_method("get_method", |lua, this, ()| {
            let state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            lua.pack(state.method.clone())
        });

        // set_method(method_id): 设置请求方法
        // 使用数字常量，如 cd.HTTP_POST, cd.HTTP_GET
        methods.add_method_mut("set_method", |_, this, method_id: u16| {
            let method = match method_id {
                HTTP_GET => "GET",
                HTTP_HEAD => "HEAD",
                HTTP_PUT => "PUT",
                HTTP_POST => "POST",
                HTTP_DELETE => "DELETE",
                HTTP_OPTIONS => "OPTIONS",
                HTTP_MKCOL => "MKCOL",
                HTTP_COPY => "COPY",
                HTTP_MOVE => "MOVE",
                HTTP_PROPFIND => "PROPFIND",
                HTTP_PROPPATCH => "PROPPATCH",
                HTTP_LOCK => "LOCK",
                HTTP_UNLOCK => "UNLOCK",
                HTTP_PATCH => "PATCH",
                HTTP_TRACE => "TRACE",
                _ => {
                    return Err(mlua::Error::external(anyhow!(
                        "Invalid method id: {}",
                        method_id
                    )));
                }
            };
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            state.method = method.to_string();
            Ok(())
        });

        // get_uri(): 返回当前 URI（完整路径+查询参数）
        methods.add_method("get_uri", |lua, this, ()| {
            let state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            lua.pack(state.build_uri())
        });

        // set_uri(uri, jump?): 设置当前 URI
        // 会解析 URI 中的查询参数
        methods.add_method_mut("set_uri", |_, this, (uri, jump): (String, Option<bool>)| {
            if uri.is_empty() {
                return Err(mlua::Error::external(anyhow!(
                    "uri argument must be a non-empty string"
                )));
            }
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            // 解析 URI，分离路径和查询参数
            match uri.split_once('?') {
                Some((path, query)) => {
                    state.uri_path = path.to_string();
                    state.uri_args = UriArgs::from_query(query);
                }
                None => {
                    state.uri_path = uri;
                    state.uri_args = UriArgs::new();
                }
            }
            state.jump = jump.unwrap_or(false);
            Ok(())
        });

        // get_uri_args(max_args?): 返回查询参数
        // 多值参数返回数组，无值参数返回 true
        // 默认 max_args=100，max_args=0 表示无限制
        methods.add_method("get_uri_args", |lua, this, max_args: Option<usize>| {
            let state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;

            let limit = max_args.unwrap_or(100);
            let table = lua.create_table()?;

            for (count, (k, v)) in state.uri_args.0.iter().enumerate() {
                if limit > 0 && count >= limit {
                    break;
                }

                if table.contains_key(k.clone())? {
                    let existing: mlua::Value = table.get(k.clone())?;
                    match existing {
                        mlua::Value::String(s) => {
                            let arr = lua.create_table()?;
                            arr.set(1, s)?;
                            arr.set(2, v.clone())?;
                            table.set(k.clone(), arr)?;
                        }
                        mlua::Value::Boolean(b) => {
                            let arr = lua.create_table()?;
                            arr.set(1, b)?;
                            arr.set(2, v.clone())?;
                            table.set(k.clone(), arr)?;
                        }
                        mlua::Value::Table(t) => {
                            let len = t.len()?;
                            t.set(len + 1, v.clone())?;
                        }
                        _ => {}
                    }
                } else if v.is_empty() {
                    table.set(k.clone(), true)?;
                } else {
                    table.set(k.clone(), v.clone())?;
                }
            }
            Ok(table)
        });

        // set_uri_args(args): 设置查询参数
        // args 可以是字符串 "a=1&b=2" 或 table {a=1, b="hello"}
        methods.add_method_mut("set_uri_args", |_, this, args: mlua::Value| {
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;

            state.uri_args = match args {
                mlua::Value::String(s) => {
                    let query = s.to_str()?;
                    UriArgs::from_query(&query)
                }
                mlua::Value::Table(t) => {
                    let mut uri_args = UriArgs::new();
                    for pair in t.pairs::<mlua::Value, mlua::Value>() {
                        let (k, v) = pair.map_err(|e| {
                            mlua::Error::external(anyhow!("Invalid uri_args table: {}", e))
                        })?;
                        match (k, v) {
                            (mlua::Value::String(key), mlua::Value::String(val)) => {
                                let key_str = key.to_str()?.to_string();
                                let val_str = val.to_str()?.to_string();
                                uri_args.0.push((key_str, val_str));
                            }
                            (mlua::Value::String(key), mlua::Value::Table(arr)) => {
                                let key_str = key.to_str()?.to_string();
                                for i in 1..=arr.len()? {
                                    if let mlua::Value::String(v) = arr.get(i)? {
                                        uri_args.0.push((key_str.clone(), v.to_str()?.to_string()));
                                    }
                                }
                            }
                            (mlua::Value::Number(key), mlua::Value::String(val)) => {
                                let key_str = key.to_string();
                                let val_str = val.to_str()?.to_string();
                                uri_args.0.push((key_str, val_str));
                            }
                            _ => {}
                        }
                    }
                    uri_args
                }
                mlua::Value::Nil => UriArgs::new(),
                _ => {
                    return Err(mlua::Error::external(anyhow!(
                        "args must be a string, table, or nil"
                    )));
                }
            };
            Ok(())
        });

        // read_body(): 读取请求体
        // 在 Candy 中，请求体在请求进入时已经自动读取
        // 此方法为 API 兼容性而存在，是空操作
        methods.add_method("read_body", |_, _, ()| {
            // 请求体已在请求处理前自动读取，此方法为空操作
            Ok(())
        });

        // get_body_data(): 获取请求体数据
        // 返回请求体的原始字节字符串
        // 如果请求体未读取、大小为 0 或已丢弃，返回 nil
        methods.add_method("get_body_data", |lua, this, ()| {
            let body = this
                .body
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
            match body.as_ref() {
                Some(bytes) if !bytes.is_empty() => {
                    lua.create_string(bytes.as_slice()).map(mlua::Value::String)
                }
                _ => Ok(mlua::Value::Nil),
            }
        });

        // discard_body(): 丢弃请求体
        // 在 Candy 中，请求体已读入内存，此方法清空请求体
        methods.add_method_mut("discard_body", |_, this, ()| {
            let mut body = this
                .body
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
            *body = None;
            Ok(())
        });

        // get_body_file(): 获取请求体临时文件名
        // Candy 不使用临时文件存储请求体，始终返回 nil
        methods.add_method("get_body_file", |_, _, ()| {
            // Candy 将请求体存储在内存中，不使用临时文件
            Ok(mlua::Value::Nil)
        });

        // set_body_data(data): 设置请求体数据
        methods.add_method_mut("set_body_data", |_, this, data: mlua::String| {
            let mut body = this
                .body
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
            *body = Some(data.as_bytes().to_vec());
            Ok(())
        });

        // set_body_file(file_name, auto_clean?): 从文件设置请求体
        // auto_clean 参数在 Candy 中被忽略（文件由用户管理）
        methods.add_method_mut(
            "set_body_file",
            |_, this, (file_name, _auto_clean): (String, Option<bool>)| {
                // 读取文件内容
                let content = std::fs::read(&file_name).map_err(|e| {
                    mlua::Error::external(anyhow!("Failed to read file '{}': {}", file_name, e))
                })?;

                let mut body = this
                    .body
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
                *body = Some(content);
                Ok(())
            },
        );

        // get_post_args(max_args?): 解析 POST 参数
        // 仅支持 application/x-www-form-urlencoded
        // 多值参数返回数组，无值参数返回 true
        methods.add_method("get_post_args", |lua, this, max_args: Option<usize>| {
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;

            // 如果尚未解析 POST 参数，则解析
            if state.post_args.is_none() {
                let body = this
                    .body
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
                match body.as_ref() {
                    Some(bytes) => {
                        let body_str = String::from_utf8_lossy(bytes);
                        state.post_args = Some(UriArgs::from_query(&body_str));
                    }
                    None => {
                        state.post_args = Some(UriArgs::new());
                    }
                }
            }

            let post_args = state.post_args.as_ref().unwrap();
            let limit = max_args.unwrap_or(100);
            let table = lua.create_table()?;

            for (count, (k, v)) in post_args.0.iter().enumerate() {
                if limit > 0 && count >= limit {
                    break;
                }

                if table.contains_key(k.clone())? {
                    let existing: mlua::Value = table.get(k.clone())?;
                    match existing {
                        mlua::Value::String(s) => {
                            let arr = lua.create_table()?;
                            arr.set(1, s)?;
                            arr.set(2, v.clone())?;
                            table.set(k.clone(), arr)?;
                        }
                        mlua::Value::Boolean(b) => {
                            let arr = lua.create_table()?;
                            arr.set(1, b)?;
                            arr.set(2, v.clone())?;
                            table.set(k.clone(), arr)?;
                        }
                        mlua::Value::Table(t) => {
                            let len = t.len()?;
                            t.set(len + 1, v.clone())?;
                        }
                        _ => {}
                    }
                } else if v.is_empty() {
                    table.set(k.clone(), true)?;
                } else {
                    table.set(k.clone(), v.clone())?;
                }
            }
            Ok(table)
        });

        // get_headers(max_headers?, raw?): 返回请求头 table
        // max_headers 默认 100，raw=false 时 key 为小写
        methods.add_method(
            "get_headers",
            |lua, this, (max_headers, raw): (Option<usize>, Option<bool>)| {
                let state = this
                    .state
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
                let headers = state
                    .headers
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock headers: {}", e)))?;

                let limit = max_headers.unwrap_or(100);
                let preserve_case = raw.unwrap_or(false);
                let table = lua.create_table()?;

                for (count, (name, value)) in headers.iter().enumerate() {
                    if limit > 0 && count >= limit {
                        break;
                    }

                    let key = if preserve_case {
                        name.as_str().to_string()
                    } else {
                        name.as_str().to_lowercase()
                    };

                    if let Ok(v) = value.to_str() {
                        if table.contains_key(key.clone())? {
                            let existing: mlua::Value = table.get(key.clone())?;
                            match existing {
                                mlua::Value::String(s) => {
                                    let arr = lua.create_table()?;
                                    arr.set(1, s)?;
                                    arr.set(2, v)?;
                                    table.set(key.clone(), arr)?;
                                }
                                mlua::Value::Table(t) => {
                                    let len = t.len()?;
                                    t.set(len + 1, v)?;
                                }
                                _ => {}
                            }
                        } else {
                            table.set(key.clone(), v)?;
                        }
                    }
                }

                // 如果不是 raw 模式，添加 __index 元方法支持多种查找方式
                if !preserve_case {
                    let metatable = lua.create_table()?;
                    let headers_clone = headers.clone();
                    metatable.set(
                        "__index",
                        lua.create_function(move |lua, (t, key): (mlua::Table, String)| {
                            // 先尝试直接查找
                            if t.contains_key(key.clone())? {
                                return t.get(key);
                            }
                            // 尝试转换为小写并替换下划线
                            let normalized = key.to_lowercase().replace('_', "-");
                            if t.contains_key(normalized.clone())? {
                                return t.get(normalized);
                            }
                            // 尝试原始 header 名称查找
                            let lower_key = key.to_lowercase();
                            for (name, _) in headers_clone.iter() {
                                if name.as_str().to_lowercase() == lower_key
                                    || name.as_str().to_lowercase().replace('-', "_") == lower_key
                                {
                                    let values: Vec<String> = headers_clone
                                        .get_all(name)
                                        .iter()
                                        .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
                                        .collect();
                                    if values.len() == 1 {
                                        return Ok(mlua::Value::String(
                                            lua.create_string(&values[0])?,
                                        ));
                                    } else if values.len() > 1 {
                                        let arr = lua.create_table()?;
                                        for (i, v) in values.iter().enumerate() {
                                            arr.set(i + 1, v.clone())?;
                                        }
                                        return Ok(mlua::Value::Table(arr));
                                    }
                                }
                            }
                            Ok(mlua::Value::Nil)
                        })?,
                    )?;
                    table.set_metatable(Some(metatable))?;
                }

                Ok(table)
            },
        );

        // clear_header(header_name): 清除指定的请求头
        methods.add_method_mut("clear_header", |_, this, header_name: String| {
            let state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            let mut headers = state
                .headers
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock headers: {}", e)))?;

            // 支持大小写不敏感查找
            let normalized = header_name.to_lowercase().replace('_', "-");
            if let Ok(header_name) = HeaderName::try_from(normalized.as_str()) {
                headers.remove(&header_name);
            }
            Ok(())
        });

        // set_header(header_name, header_value): 设置请求头
        // header_value 可以是字符串、数组或 nil
        // nil 表示删除 header
        methods.add_method_mut(
            "set_header",
            |_, this, (header_name, header_value): (String, mlua::Value)| {
                let state = this
                    .state
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
                let mut headers = state
                    .headers
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock headers: {}", e)))?;

                let normalized = header_name.to_lowercase().replace('_', "-");
                let header_name = HeaderName::try_from(normalized.as_str())
                    .map_err(|e| mlua::Error::external(anyhow!("Invalid header name: {}", e)))?;

                match header_value {
                    mlua::Value::Nil => {
                        // nil 表示删除 header
                        headers.remove(&header_name);
                    }
                    mlua::Value::String(s) => {
                        let header_value = HeaderValue::from_str(&s.to_str()?).map_err(|e| {
                            mlua::Error::external(anyhow!("Invalid header value: {}", e))
                        })?;
                        headers.insert(header_name, header_value);
                    }
                    mlua::Value::Table(t) => {
                        // 数组值：先删除旧的，再添加所有新值
                        headers.remove(&header_name);
                        for i in 1..=t.len()? {
                            if let mlua::Value::String(s) = t.get(i)? {
                                let header_value =
                                    HeaderValue::from_str(&s.to_str()?).map_err(|e| {
                                        mlua::Error::external(anyhow!(
                                            "Invalid header value: {}",
                                            e
                                        ))
                                    })?;
                                headers.append(&header_name, header_value);
                            }
                        }
                    }
                    _ => {
                        return Err(mlua::Error::external(anyhow!(
                            "header_value must be a string, table, or nil"
                        )));
                    }
                }
                Ok(())
            },
        );
    }
}

impl UserData for CandyHeaders {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // __index: 读取 header
        // 支持 cd.header["Content-Type"] 和 cd.header.content_type
        methods.add_meta_method("__index", |lua, this, key: String| {
            let normalized = Self::normalize_header_name(&key);
            let headers = this
                .headers
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock headers: {}", e)))?;

            // 查找 header (大小写不敏感)
            let header_name = HeaderName::try_from(normalized.as_str())
                .map_err(|e| mlua::Error::external(anyhow!("Invalid header name: {}", e)))?;

            let values: Vec<String> = headers
                .get_all(&header_name)
                .iter()
                .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
                .collect();

            if values.is_empty() {
                Ok(mlua::Value::Nil)
            } else if values.len() == 1 {
                Ok(mlua::Value::String(lua.create_string(&values[0])?))
            } else {
                // 多值 header 返回 table
                let table = lua.create_table()?;
                for (i, v) in values.iter().enumerate() {
                    table.set(i + 1, v.clone())?;
                }
                Ok(mlua::Value::Table(table))
            }
        });

        // __newindex: 设置/删除 header
        // cd.header["Content-Type"] = "text/plain"
        // cd.header["Set-Cookie"] = {"a=1", "b=2"}
        // cd.header["X-My-Header"] = nil  -- 删除
        methods.add_meta_method_mut(
            "__newindex",
            |_lua, this, (key, value): (String, mlua::Value)| {
                let normalized = Self::normalize_header_name(&key);
                let header_name = HeaderName::try_from(normalized.as_str())
                    .map_err(|e| mlua::Error::external(anyhow!("Invalid header name: {}", e)))?;

                let mut headers = this
                    .headers
                    .lock()
                    .map_err(|e| mlua::Error::external(anyhow!("Failed to lock headers: {}", e)))?;

                // 先移除已有的值
                headers.remove(&header_name);

                match value {
                    mlua::Value::Nil => {
                        // 删除 header，已经 remove 了，不需要额外操作
                    }
                    mlua::Value::String(s) => {
                        let val = s.to_str()?;
                        let header_value = HeaderValue::from_str(&val).map_err(|e| {
                            mlua::Error::external(anyhow!("Invalid header value: {}", e))
                        })?;
                        headers.append(header_name.clone(), header_value);
                    }
                    mlua::Value::Table(t) => {
                        // 多值 header
                        for pair in t.pairs::<i32, mlua::String>() {
                            let (_, v) = pair.map_err(|e| {
                                mlua::Error::external(anyhow!(
                                    "Invalid header value in table: {}",
                                    e
                                ))
                            })?;
                            let val = v.to_str()?;
                            let header_value = HeaderValue::from_str(&val).map_err(|e| {
                                mlua::Error::external(anyhow!("Invalid header value: {}", e))
                            })?;
                            headers.append(header_name.clone(), header_value);
                        }
                    }
                    _ => {
                        return Err(mlua::Error::external(anyhow!(
                            "Header value must be string, table, or nil"
                        )));
                    }
                }

                Ok(())
            },
        );
    }
}

impl UserData for RequestContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // 元方法：实现属性访问 (cd.status, cd.header)
        // 注意：需要同时处理常量字段和动态属性
        methods.add_meta_method("__index", |lua, this, key: String| {
            match key.as_str() {
                // 动态属性
                "status" => lua.pack(this.res.status),
                "header" => {
                    // 返回 headers 对象
                    lua.create_userdata(this.res.headers.clone())
                        .map(mlua::Value::UserData)
                }
                "resp" => {
                    // 返回 resp 对象，提供 get_headers 方法
                    lua.create_userdata(CandyResp {
                        headers: this.res.headers.clone(),
                    })
                    .map(mlua::Value::UserData)
                }
                "req" => {
                    // 返回 req 对象，提供 is_internal 等方法
                    lua.create_userdata(CandyReq {
                        is_internal: false,
                        start_time: this.start_time,
                        http_version: this.req.http_version,
                        raw_header: this.req.raw_header.clone(),
                        request_line: this.req.request_line.clone(),
                        body: this.req.body.clone(),
                        state: this.req_state.clone(),
                    })
                    .map(mlua::Value::UserData)
                }
                "now" => {
                    // now(): 返回当前时间戳（秒，包含毫秒小数部分）
                    let now_func = lua.create_function(|lua, ()| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        let secs =
                            now.as_secs() as f64 + now.subsec_nanos() as f64 / 1_000_000_000.0;
                        lua.pack(secs)
                    })?;
                    Ok(mlua::Value::Function(now_func))
                }
                "time" => {
                    // time(): 返回当前时间戳（整数秒）
                    let time_func = lua.create_function(|lua, ()| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        lua.pack(now.as_secs())
                    })?;
                    Ok(mlua::Value::Function(time_func))
                }
                "today" => {
                    // today(): 返回当前日期（格式 yyyy-mm-dd）
                    let today_func = lua.create_function(|lua, ()| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        let secs = now.as_secs();
                        // 计算日期（简化实现，不处理时区）
                        let days = secs / 86400;
                        // 从 1970-01-01 开始计算
                        let (year, month, day) = super::utils::days_to_ymd(days as i32);
                        let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
                        lua.pack(date_str)
                    })?;
                    Ok(mlua::Value::Function(today_func))
                }
                "update_time" => {
                    // update_time(): 强制更新时间（在 Candy 中是空操作，因为每次都获取最新时间）
                    let update_time_func = lua.create_function(|_, ()| {
                        // Candy 每次调用 now()/today() 都会获取最新时间
                        // 此函数仅为 API 兼容性而存在
                        Ok(())
                    })?;
                    Ok(mlua::Value::Function(update_time_func))
                }
                // HTTP 方法常量
                "HTTP_GET" => lua.pack(HTTP_GET),
                "HTTP_HEAD" => lua.pack(HTTP_HEAD),
                "HTTP_PUT" => lua.pack(HTTP_PUT),
                "HTTP_POST" => lua.pack(HTTP_POST),
                "HTTP_DELETE" => lua.pack(HTTP_DELETE),
                "HTTP_OPTIONS" => lua.pack(HTTP_OPTIONS),
                "HTTP_MKCOL" => lua.pack(HTTP_MKCOL),
                "HTTP_COPY" => lua.pack(HTTP_COPY),
                "HTTP_MOVE" => lua.pack(HTTP_MOVE),
                "HTTP_PROPFIND" => lua.pack(HTTP_PROPFIND),
                "HTTP_PROPPATCH" => lua.pack(HTTP_PROPPATCH),
                "HTTP_LOCK" => lua.pack(HTTP_LOCK),
                "HTTP_UNLOCK" => lua.pack(HTTP_UNLOCK),
                "HTTP_PATCH" => lua.pack(HTTP_PATCH),
                "HTTP_TRACE" => lua.pack(HTTP_TRACE),
                // HTTP 状态码常量 - 1xx
                "HTTP_CONTINUE" => lua.pack(HTTP_CONTINUE),
                "HTTP_SWITCHING_PROTOCOLS" => lua.pack(HTTP_SWITCHING_PROTOCOLS),
                // HTTP 状态码常量 - 2xx
                "HTTP_OK" => lua.pack(HTTP_OK),
                "HTTP_CREATED" => lua.pack(HTTP_CREATED),
                "HTTP_ACCEPTED" => lua.pack(HTTP_ACCEPTED),
                "HTTP_NO_CONTENT" => lua.pack(HTTP_NO_CONTENT),
                "HTTP_PARTIAL_CONTENT" => lua.pack(HTTP_PARTIAL_CONTENT),
                // HTTP 状态码常量 - 3xx
                "HTTP_SPECIAL_RESPONSE" => lua.pack(HTTP_SPECIAL_RESPONSE),
                "HTTP_MOVED_PERMANENTLY" => lua.pack(HTTP_MOVED_PERMANENTLY),
                "HTTP_MOVED_TEMPORARILY" => lua.pack(HTTP_MOVED_TEMPORARILY),
                "HTTP_SEE_OTHER" => lua.pack(HTTP_SEE_OTHER),
                "HTTP_NOT_MODIFIED" => lua.pack(HTTP_NOT_MODIFIED),
                "HTTP_TEMPORARY_REDIRECT" => lua.pack(HTTP_TEMPORARY_REDIRECT),
                // HTTP 状态码常量 - 4xx
                "HTTP_BAD_REQUEST" => lua.pack(HTTP_BAD_REQUEST),
                "HTTP_UNAUTHORIZED" => lua.pack(HTTP_UNAUTHORIZED),
                "HTTP_PAYMENT_REQUIRED" => lua.pack(HTTP_PAYMENT_REQUIRED),
                "HTTP_FORBIDDEN" => lua.pack(HTTP_FORBIDDEN),
                "HTTP_NOT_FOUND" => lua.pack(HTTP_NOT_FOUND),
                "HTTP_NOT_ALLOWED" => lua.pack(HTTP_NOT_ALLOWED),
                "HTTP_NOT_ACCEPTABLE" => lua.pack(HTTP_NOT_ACCEPTABLE),
                "HTTP_REQUEST_TIMEOUT" => lua.pack(HTTP_REQUEST_TIMEOUT),
                "HTTP_CONFLICT" => lua.pack(HTTP_CONFLICT),
                "HTTP_GONE" => lua.pack(HTTP_GONE),
                "HTTP_UPGRADE_REQUIRED" => lua.pack(HTTP_UPGRADE_REQUIRED),
                "HTTP_TOO_MANY_REQUESTS" => lua.pack(HTTP_TOO_MANY_REQUESTS),
                "HTTP_CLOSE" => lua.pack(HTTP_CLOSE),
                "HTTP_ILLEGAL" => lua.pack(HTTP_ILLEGAL),
                // HTTP 状态码常量 - 5xx
                "HTTP_INTERNAL_SERVER_ERROR" => lua.pack(HTTP_INTERNAL_SERVER_ERROR),
                "HTTP_METHOD_NOT_IMPLEMENTED" => lua.pack(HTTP_METHOD_NOT_IMPLEMENTED),
                "HTTP_BAD_GATEWAY" => lua.pack(HTTP_BAD_GATEWAY),
                "HTTP_SERVICE_UNAVAILABLE" => lua.pack(HTTP_SERVICE_UNAVAILABLE),
                "HTTP_GATEWAY_TIMEOUT" => lua.pack(HTTP_GATEWAY_TIMEOUT),
                "HTTP_VERSION_NOT_SUPPORTED" => lua.pack(HTTP_VERSION_NOT_SUPPORTED),
                "HTTP_INSUFFICIENT_STORAGE" => lua.pack(HTTP_INSUFFICIENT_STORAGE),
                _ => Err(mlua::Error::external(anyhow!(
                    "attempt to index unknown field: {}",
                    key
                ))),
            }
        });

        // 元方法：实现属性设置 (cd.status = 200)
        methods.add_meta_method_mut(
            "__newindex",
            |_, this, (key, value): (String, u16)| match key.as_str() {
                "status" => {
                    this.res.status = value;
                    Ok(())
                }
                _ => Err(mlua::Error::external(anyhow!(
                    "attempt to set unknown field: {}",
                    key
                ))),
            },
        );
    }
}
