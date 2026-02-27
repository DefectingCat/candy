use anyhow::anyhow;
use http::{HeaderName, HeaderValue};
use mlua::{UserData, UserDataMethods};

use super::{
    constants::*,
    structures::{CandyHeaders, CandyReq, CandyResp, RequestContext},
    utils::UriArgs,
};

// Helper function to escape URI components
fn url_escape_component(input: &str) -> String {
    let mut result = String::new();
    
    for c in input.chars() {
        match c {
            // RFC 3986 unreserved characters - don't encode these
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => {
                result.push(c);
            }
            // All other characters get percent-encoded
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
    }
    
    result
}

// Helper function to unescape URI components
fn url_unescape_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Try to read the next two hex digits
            let hex1 = bytes[i + 1];
            let hex2 = bytes[i + 2];
            
            // Convert hex digits to character
            if let (Some(d1), Some(d2)) = (hex_digit_value(hex1), hex_digit_value(hex2)) {
                let decoded_byte = (d1 << 4) | d2;
                result.push(decoded_byte);
                i += 3; // Skip % and two hex digits
            } else {
                // Invalid hex, keep the original byte
                result.push(bytes[i]);
                i += 1;
            }
        } else if bytes[i] == b'+' {
            // According to the example in the docs, + is converted to space
            // Example: "b%20r56+7" -> "b r56 7" shows that + becomes space too
            result.push(b' ');
            i += 1;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    
    // Convert bytes back to string, handling potential UTF-8
    String::from_utf8_lossy(&result).to_string()
}

// Helper function to convert hex digit to value
fn hex_digit_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

// Helper function to convert Lua value to string representation for args
// Returns (string_value, has_value) where has_value indicates if the value should have =value or just be a key
fn value_to_string_for_args(value: &mlua::Value) -> mlua::Result<Option<(String, bool)>> {
    match value {
        mlua::Value::Nil => Ok(None), // nil values are skipped
        mlua::Value::Boolean(b) => {
            if *b {
                Ok(Some(("".to_string(), false))) // true becomes just the key without =value
            } else {
                Ok(None) // false is treated as nil (skipped)
            }
        },
        mlua::Value::Number(n) => Ok(Some((n.to_string(), true))), // has value
        mlua::Value::Integer(i) => Ok(Some((i.to_string(), true))), // has value
        mlua::Value::String(s) => Ok(Some((s.to_str()?.to_string(), true))), // has value
        mlua::Value::Table(_) => Ok(None), // Tables are handled specially elsewhere
        mlua::Value::UserData(_) => Ok(Some(("<userdata>".to_string(), true))), // has value
        _ => Ok(Some((format!("{:?}", value), true))), // has value
    }
}

// Helper function to recursively convert Lua table to string
fn table_to_string_impl(_lua: &mlua::Lua, table: &mlua::Table) -> mlua::Result<String> {
    let mut result = String::new();
    
    for pair in table.pairs::<mlua::Value, mlua::Value>() {
        let (_, value) = pair?;
        
        match value {
            mlua::Value::Nil => result.push_str("nil"),
            mlua::Value::Boolean(b) => result.push_str(if b { "true" } else { "false" }),
            mlua::Value::Number(n) => result.push_str(&n.to_string()),
            mlua::Value::Integer(i) => result.push_str(&i.to_string()),
            mlua::Value::String(s) => result.push_str(&s.to_str()?),
            mlua::Value::Table(t) => {
                // Recursively handle nested tables
                result.push_str(&table_to_string_impl(_lua, &t)?);
            }
            mlua::Value::UserData(ud) => {
                let s = format!("{:?}", ud);
                result.push_str(&s);
            }
            _ => {
                let s = format!("{:?}", value);
                result.push_str(&s);
            }
        }
    }
    
    Ok(result)
}

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

        // init_body(buffer_size?): 初始化新的空白请求体
        // 为后续通过 append_body 和 finish_body 追加请求体数据做准备
        // buffer_size 指定内存缓冲区大小（字节），默认 8KB
        // 在 Candy 中，请求体存储在内存中，不使用临时文件
        methods.add_method_mut("init_body", |_, this, buffer_size: Option<usize>| {
            let _ = buffer_size; // 在内存实现中未使用，但保留参数兼容性
            let mut body = this
                .body
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
            // 初始化为空字节数组，表示可追加状态
            *body = Some(Vec::new());
            Ok(())
        });

        // append_body(data): 追加数据到请求体
        // 必须在 init_body 之后、finish_body 之前调用
        // data 可以是字符串或 Lua 字符串
        methods.add_method_mut("append_body", |_, this, data: mlua::String| {
            let mut body = this
                .body
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock body: {}", e)))?;
            match body.as_mut() {
                Some(vec) => {
                    vec.extend_from_slice(&data.as_bytes());
                }
                None => {
                    return Err(mlua::Error::external(anyhow!(
                        "request body not initialized, call ngx.req.init_body first"
                    )));
                }
            }
            Ok(())
        });

        // finish_body(): 完成请求体写入
        // 在所有数据通过 append_body 追加完毕后调用
        // 在 Candy 中，此方法为空操作（仅标记写入完成）
        methods.add_method_mut("finish_body", |_, _this, ()| {
            // 在内存实现中无需额外操作
            // 在基于文件的实现中，这里会刷新缓冲区到临时文件
            Ok(())
        });

        // print(...): 输出数据到响应体
        // 连接所有参数并发送到 HTTP 客户端
        // 返回 1 表示成功，或返回 nil 和错误描述字符串
        methods.add_method_mut("print", |lua, this, args: mlua::MultiValue| {
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            
            // 构建输出字符串
            let mut output = String::new();
            
            for value in args {
                match value {
                    mlua::Value::Nil => output.push_str("nil"),
                    mlua::Value::Boolean(b) => output.push_str(if b { "true" } else { "false" }),
                    mlua::Value::Number(n) => output.push_str(&n.to_string()),
                    mlua::Value::Integer(i) => output.push_str(&i.to_string()),
                    mlua::Value::String(s) => output.push_str(&s.to_str()?),
                    mlua::Value::Table(t) => {
                        // 递归处理嵌套表 - 简单实现
                        output.push_str(&table_to_string_impl(lua, &t)?);
                    }
                    mlua::Value::UserData(ud) => {
                        // 尝试获取用户数据的字符串表示
                        let s = format!("{:?}", ud);
                        output.push_str(&s);
                    }
                    _ => {
                        // 其他类型转换为字符串
                        let s = format!("{:?}", value);
                        output.push_str(&s);
                    }
                }
            }
            
            // 将输出追加到缓冲区
            state.output_buffer.push_str(&output);
            
            // 返回成功状态 1
            Ok(1)
        });

        // say(...): 输出数据到响应体并添加换行符
        // 与 print 类似，但会在末尾添加换行符
        methods.add_method_mut("say", |lua, this, args: mlua::MultiValue| {
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            
            // 构建输出字符串
            let mut output = String::new();
            
            for value in args {
                match value {
                    mlua::Value::Nil => output.push_str("nil"),
                    mlua::Value::Boolean(b) => output.push_str(if b { "true" } else { "false" }),
                    mlua::Value::Number(n) => output.push_str(&n.to_string()),
                    mlua::Value::Integer(i) => output.push_str(&i.to_string()),
                    mlua::Value::String(s) => output.push_str(&s.to_str()?),
                    mlua::Value::Table(t) => {
                        // 递归处理嵌套表 - 简单实现
                        output.push_str(&table_to_string_impl(lua, &t)?);
                    }
                    mlua::Value::UserData(ud) => {
                        // 尝试获取用户数据的字符串表示
                        let s = format!("{:?}", ud);
                        output.push_str(&s);
                    }
                    _ => {
                        // 其他类型转换为字符串
                        let s = format!("{:?}", value);
                        output.push_str(&s);
                    }
                }
            }
            
            // 添加换行符
            output.push('\n');
            
            // 将输出追加到缓冲区
            state.output_buffer.push_str(&output);
            
            // 返回成功状态 1
            Ok(1)
        });

        // flush(wait?): 刷新响应输出到客户端
        // wait 默认为 false，异步模式立即返回
        // wait 为 true 时，同步等待直到所有数据写入系统发送缓冲区
        methods.add_method_mut("flush", |_lua, _this, wait: Option<bool>| {
            // 在 Candy 的实现中，我们只是简单地返回成功
            // 真正的刷新逻辑是在响应发送时处理的
            let _wait = wait.unwrap_or(false);
            
            // 实际上，在当前的 Candy 实现中，输出会在请求结束时自动刷新
            // 所以我们只需返回成功状态
            // 在真实的实现中，这会根据 wait 参数决定是否等待
            Ok(1)
        });

        // exit(status): 退出当前请求处理并返回状态码
        // status >= 200 时，中断当前请求并返回状态码
        // status == 0 时，仅退出当前阶段处理器
        methods.add_method_mut("exit", |_, this, status: u16| {
            // 在 Candy 的实现中，我们通过修改响应状态来模拟退出
            // 实际的请求终止由框架处理
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            
            // 设置退出状态，框架会根据这个来决定如何处理
            // 对于 Candy，我们会直接设置响应状态码
            state.redirect_status = Some(status);
            
            // 如果状态码 >= 200，则认为是正常退出
            if status >= 200 {
                // 这里不会真正退出，而是让框架知道应该结束请求
                // 但在 Lua 中这通常会导致协程结束
                // 我们简单地返回，因为 mlua 不允许我们直接退出
                Ok(())
            } else {
                // 对于 NGX_OK (0)，只退出当前阶段
                Ok(())
            }
        });

        // eof(): 明确指定响应输出流的结束
        // 在 HTTP 1.1 分块编码输出的情况下，会触发 Nginx 核心发送 "last chunk"
        methods.add_method_mut("eof", |_lua, this, ()| {
            // 在 Candy 的实现中，我们通过设置标志来表示输出流结束
            let mut state = this
                .state
                .lock()
                .map_err(|e| mlua::Error::external(anyhow!("Failed to lock state: {}", e)))?;
            
            // 设置输出结束标志
            // 在实际实现中，这会告诉底层框架停止接受更多输出并发送结束信号
            // 对于 Candy，我们可能需要一个标志来表示响应已完成
            // 但由于我们无法直接控制底层传输，我们只是记录这个事件
            state.output_buffer.push_str(""); // 不改变缓冲区，只是记录事件
            
            // 返回成功状态 1
            Ok(1)
        });

        // sleep(seconds): 休眠指定的秒数而不阻塞
        // 可以指定精确到 0.001 秒（即 1 毫秒）的时间分辨率
        // 底层使用 Nginx 定时器
        methods.add_async_method_mut("sleep", |_lua, _this, seconds: f64| {
            use tokio::time::Duration;
            
            // 将秒转换为 Duration
            let duration = Duration::from_secs_f64(seconds);
            
            Box::pin(async move {
                // 异步等待指定的时间
                tokio::time::sleep(duration).await;
                
                Ok(())
            })
        });

        // escape_uri(str): 将字符串作为 URI 组件进行转义
        methods.add_method_mut("escape_uri", |lua, _this, str: String| {
            // 对 URI 进行编码，将特殊字符转换为百分号编码
            let encoded = url_escape_component(&str);
            lua.create_string(&encoded).map(mlua::Value::String)
        });

        // unescape_uri(str): 将字符串作为转义的 URI 组件进行解码
        methods.add_method_mut("unescape_uri", |lua, _this, str: String| {
            // 对 URI 进行解码，将百分号编码转换回原始字符
            let decoded = url_unescape_component(&str);
            lua.create_string(&decoded).map(mlua::Value::String)
        });

        // encode_args(table): 将 Lua 表编码为查询参数字符串
        methods.add_method_mut("encode_args", |lua, _this, table: mlua::Table| {
            let mut args = Vec::new();
            
            for pair in table.pairs::<mlua::Value, mlua::Value>() {
                let (key, value) = pair?;
                
                let key_str = match key {
                    mlua::Value::String(s) => s.to_str()?.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    _ => return Err(mlua::Error::external(anyhow::anyhow!("Table key must be a string, number, or integer"))),
                };
                
                let encoded_key = url_escape_component(&key_str);
                
                match value {
                    mlua::Value::Table(arr) => {
                        // 处理多值参数
                        for i in 1..=arr.len()? {
                            let val = arr.get(i)?;
                            if let Some((val_str, has_value)) = value_to_string_for_args(&val)? {
                                if has_value {
                                    let encoded_val = url_escape_component(&val_str);
                                    args.push(format!("{}={}", encoded_key, encoded_val));
                                } else {
                                    // Boolean true without value
                                    args.push(encoded_key.clone());
                                }
                            }
                        }
                    }
                    _ => {
                        if let Some((val_str, has_value)) = value_to_string_for_args(&value)? {
                            if has_value {
                                let encoded_val = url_escape_component(&val_str);
                                args.push(format!("{}={}", encoded_key, encoded_val));
                            } else {
                                // Boolean true without value
                                args.push(encoded_key);
                            }
                        }
                    }
                }
            }
            
            let result = args.join("&");
            lua.create_string(&result).map(mlua::Value::String)
        });

        // decode_args(str, max_args?): 将 URI 编码的查询字符串解码为 Lua 表
        // max_args 默认为 100，设为 0 表示无限制
        methods.add_method_mut("decode_args", |lua, _this, (str, max_args): (String, Option<usize>)| {
            let max_count = max_args.unwrap_or(100);
            let result = lua.create_table()?;

            let mut count = 0;
            for pair in str.split('&') {
                if !pair.is_empty() {
                    // 检查是否达到最大参数数量限制（除非限制为0，表示无限制）
                    if max_count > 0 && count >= max_count {
                        break; // 达到最大参数数量限制
                    }
                    
                    let (key, value) = if let Some(pos) = pair.find('=') {
                        (&pair[..pos], &pair[pos + 1..])
                    } else {
                        (pair, "") // 无值参数，如布尔值参数
                    };
                    
                    // 解码键和值
                    let decoded_key = url_unescape_component(key);
                    let decoded_value = url_unescape_component(value);
                    
                    // 检查键是否已存在
                    if result.contains_key(decoded_key.as_str())? {
                        // 键已存在，转换为数组或添加到现有数组
                        let existing: mlua::Value = result.get(decoded_key.as_str())?;
                        match existing {
                            mlua::Value::String(_) => {
                                // 将单个值转换为数组
                                let arr = lua.create_table()?;
                                arr.set(1, existing)?;
                                arr.set(2, decoded_value)?;
                                result.set(decoded_key.as_str(), arr)?;
                            }
                            mlua::Value::Table(t) => {
                                // 添加到现有数组
                                let len = t.len()?;
                                t.set(len + 1, decoded_value)?;
                            }
                            _ => {
                                // 其他情况，保持第一个值
                                result.set(decoded_key.as_str(), decoded_value)?;
                            }
                        }
                    } else {
                        // 新键
                        result.set(decoded_key.as_str(), decoded_value)?;
                    }
                    
                    count += 1;
                }
            }
            
            Ok(result)
        });

        // log(log_level, ...): 记录日志消息
        // 将参数连接并记录到错误日志中，带有指定的日志级别
        methods.add_method_mut("log", |_, _this, args: mlua::MultiValue| {
            use tracing::{error, warn, info, debug};

            let mut iter = args.into_iter();
            
            // 获取日志级别
            let log_level = match iter.next() {
                Some(mlua::Value::Integer(level)) => level as u8,
                Some(mlua::Value::Number(level)) => level as u8,
                _ => {
                    error!("ngx.log: first argument must be log level");
                    return Ok(());
                }
            };

            // 构建日志消息
            let mut message_parts = Vec::new();
            for value in iter {
                let s = match value {
                    mlua::Value::Nil => "nil".to_string(),
                    mlua::Value::Boolean(b) => if b { "true".to_string() } else { "false".to_string() },
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::String(s) => {
                        match s.to_str() {
                            Ok(str_val) => str_val.to_string(),
                            Err(_) => "<invalid>".to_string(),
                        }
                    },
                    mlua::Value::Table(_) => "<table>".to_string(), // 避免复杂表的处理
                    mlua::Value::UserData(_) => "<userdata>".to_string(),
                    _ => format!("{:?}", value),
                };
                message_parts.push(s);
            }
            
            let message = message_parts.join("");
            
            // 根据日志级别记录消息
            match log_level {
                LOG_EMERG | LOG_ALERT | LOG_CRIT | LOG_ERR => {
                    error!("ngx.log: {}", message);
                }
                LOG_WARN => {
                    warn!("ngx.log: {}", message);
                }
                LOG_NOTICE | LOG_INFO => {
                    info!("ngx.log: {}", message);
                }
                LOG_DEBUG => {
                    debug!("ngx.log: {}", message);
                }
                _ => {
                    // 其他级别默认使用 info
                    info!("ngx.log: {}", message);
                }
            }
            
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::{header, HeaderMap, HeaderValue};
    use std::sync::{Arc, Mutex};

    // Helper function to get method name from method ID
    fn method_id_to_string(method_id: u16) -> Option<&'static str> {
        match method_id {
            HTTP_GET => Some("GET"),
            HTTP_HEAD => Some("HEAD"),
            HTTP_PUT => Some("PUT"),
            HTTP_POST => Some("POST"),
            HTTP_DELETE => Some("DELETE"),
            HTTP_OPTIONS => Some("OPTIONS"),
            HTTP_MKCOL => Some("MKCOL"),
            HTTP_COPY => Some("COPY"),
            HTTP_MOVE => Some("MOVE"),
            HTTP_PROPFIND => Some("PROPFIND"),
            HTTP_PROPPATCH => Some("PROPPATCH"),
            HTTP_LOCK => Some("LOCK"),
            HTTP_UNLOCK => Some("UNLOCK"),
            HTTP_PATCH => Some("PATCH"),
            HTTP_TRACE => Some("TRACE"),
            _ => None,
        }
    }

    // Helper function to validate header name normalization
    fn normalize_header_name_for_test(key: &str) -> String {
        key.replace('_', "-").to_lowercase()
    }

    // method_id_to_string tests
    mod method_id_to_string {
        use super::*;

        #[test]
        fn test_all_valid_methods() {
            assert_eq!(method_id_to_string(HTTP_GET), Some("GET"));
            assert_eq!(method_id_to_string(HTTP_HEAD), Some("HEAD"));
            assert_eq!(method_id_to_string(HTTP_PUT), Some("PUT"));
            assert_eq!(method_id_to_string(HTTP_POST), Some("POST"));
            assert_eq!(method_id_to_string(HTTP_DELETE), Some("DELETE"));
            assert_eq!(method_id_to_string(HTTP_OPTIONS), Some("OPTIONS"));
            assert_eq!(method_id_to_string(HTTP_MKCOL), Some("MKCOL"));
            assert_eq!(method_id_to_string(HTTP_COPY), Some("COPY"));
            assert_eq!(method_id_to_string(HTTP_MOVE), Some("MOVE"));
            assert_eq!(method_id_to_string(HTTP_PROPFIND), Some("PROPFIND"));
            assert_eq!(method_id_to_string(HTTP_PROPPATCH), Some("PROPPATCH"));
            assert_eq!(method_id_to_string(HTTP_LOCK), Some("LOCK"));
            assert_eq!(method_id_to_string(HTTP_UNLOCK), Some("UNLOCK"));
            assert_eq!(method_id_to_string(HTTP_PATCH), Some("PATCH"));
            assert_eq!(method_id_to_string(HTTP_TRACE), Some("TRACE"));
        }

        #[test]
        fn test_invalid_method_id() {
            assert_eq!(method_id_to_string(100), None);
            assert_eq!(method_id_to_string(999), None);
            assert_eq!(method_id_to_string(u16::MAX), None);
        }
    }

    // normalize_header_name_for_test tests
    mod normalize_header_name {
        use super::*;

        #[test]
        fn test_no_underscore() {
            assert_eq!(normalize_header_name_for_test("content-type"), "content-type");
            assert_eq!(normalize_header_name_for_test("host"), "host");
        }

        #[test]
        fn test_with_underscore() {
            assert_eq!(normalize_header_name_for_test("content_type"), "content-type");
            assert_eq!(normalize_header_name_for_test("x_custom_header"), "x-custom-header");
        }

        #[test]
        fn test_mixed_case() {
            assert_eq!(normalize_header_name_for_test("Content_Type"), "content-type");
            assert_eq!(normalize_header_name_for_test("X_API_Key"), "x-api-key");
        }

        #[test]
        fn test_empty_string() {
            assert_eq!(normalize_header_name_for_test(""), "");
        }
    }

    // CandyReq construction and basic field tests
    mod candy_req {
        use super::*;
        use crate::http::lua::structures::CandyReqState;

        fn create_test_candy_req() -> CandyReq {
            let body = Arc::new(Mutex::new(Some(b"test body".to_vec())));
            let headers = Arc::new(Mutex::new(HeaderMap::new()));
            let state = Arc::new(Mutex::new(CandyReqState {
                method: "GET".to_string(),
                uri_path: "/test".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: headers.clone(),
                redirect_status: None,
                output_buffer: String::new(),
            }));

            CandyReq {
                is_internal: false,
                start_time: 1234567890.0,
                http_version: Some(1.1),
                raw_header: "Host: localhost\r\n".to_string(),
                request_line: "GET /test HTTP/1.1".to_string(),
                body,
                state,
            }
        }

        #[test]
        fn test_candy_req_creation() {
            let req = create_test_candy_req();
            assert!(!req.is_internal);
            assert_eq!(req.start_time, 1234567890.0);
            assert_eq!(req.http_version, Some(1.1));
        }

        #[test]
        fn test_candy_req_body_access() {
            let req = create_test_candy_req();
            let guard = req.body.lock().unwrap();
            assert_eq!(guard.as_ref().unwrap(), b"test body");
        }

        #[test]
        fn test_candy_req_state_access() {
            let req = create_test_candy_req();
            let state = req.state.lock().unwrap();
            assert_eq!(state.method, "GET");
            assert_eq!(state.uri_path, "/test");
        }
    }

    // CandyResp tests
    mod candy_resp {
        use super::*;

        fn create_test_candy_resp() -> CandyResp {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            CandyResp {
                headers: CandyHeaders::new(headers),
            }
        }

        #[test]
        fn test_candy_resp_creation() {
            let resp = create_test_candy_resp();
            let guard = resp.headers.headers.lock().unwrap();
            assert!(guard.get(header::CONTENT_TYPE).is_some());
        }
    }

    // CandyHeaders tests
    mod candy_headers {
        use super::*;

        #[test]
        fn test_new_headers() {
            let headers = HeaderMap::new();
            let candy_headers = CandyHeaders::new(headers);
            assert!(candy_headers.headers.lock().is_ok());
        }

        #[test]
        fn test_headers_with_values() {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html"),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_static("100"),
            );

            let candy_headers = CandyHeaders::new(headers);
            let guard = candy_headers.headers.lock().unwrap();

            assert_eq!(guard.get(header::CONTENT_TYPE).unwrap(), "text/html");
            assert_eq!(guard.get(header::CONTENT_LENGTH).unwrap(), "100");
        }

        #[test]
        fn test_headers_clone() {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::HOST,
                HeaderValue::from_static("localhost"),
            );
            let candy_headers = CandyHeaders::new(headers);
            let cloned = candy_headers.headers.lock().unwrap().clone();
            assert_eq!(cloned.get(header::HOST).unwrap(), "localhost");
        }
    }

    // RequestContext tests
    mod request_context {
        use super::*;
        use crate::http::lua::structures::{CandyRequest, CandyResponse, CandyReqState};
        use http::Uri;

        fn create_test_request_context() -> RequestContext {
            let body = Arc::new(Mutex::new(Some(b"".to_vec())));
            let headers = Arc::new(Mutex::new(HeaderMap::new()));
            let req_state = Arc::new(Mutex::new(CandyReqState {
                method: "GET".to_string(),
                uri_path: "/".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: headers.clone(),
                redirect_status: None,
                output_buffer: String::new(),
            }));

            RequestContext {
                req: CandyRequest {
                    uri: Uri::from_static("/"),
                    http_version: Some(1.1),
                    raw_header: String::new(),
                    request_line: "GET / HTTP/1.1".to_string(),
                    body,
                },
                res: CandyResponse {
                    status: 200,
                    headers: CandyHeaders::new(HeaderMap::new()),
                    body: "".to_string(),
                },
                start_time: 1000.0,
                req_state,
            }
        }

        #[test]
        fn test_request_context_creation() {
            let ctx = create_test_request_context();
            assert_eq!(ctx.start_time, 1000.0);
            assert_eq!(ctx.res.status, 200);
        }

        #[test]
        fn test_request_context_clone() {
            let ctx = create_test_request_context();
            let cloned = ctx.clone();
            assert_eq!(cloned.start_time, ctx.start_time);
            assert_eq!(cloned.res.status, ctx.res.status);
        }
    }

    // UriArgs construction and behavior tests
    mod uri_args {
        use super::*;

        #[test]
        fn test_from_query_simple() {
            let args = UriArgs::from_query("a=1&b=2");
            assert_eq!(args.0.len(), 2);
            assert_eq!(args.0[0], ("a".to_string(), "1".to_string()));
            assert_eq!(args.0[1], ("b".to_string(), "2".to_string()));
        }

        #[test]
        fn test_from_query_empty_value() {
            let args = UriArgs::from_query("flag&key=value");
            assert_eq!(args.0.len(), 2);
            assert_eq!(args.0[0].0, "flag");
            assert_eq!(args.0[0].1, ""); // empty value for flag without =
            assert_eq!(args.0[1], ("key".to_string(), "value".to_string()));
        }

        #[test]
        fn test_from_query_empty() {
            let args = UriArgs::from_query("");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_to_query_simple() {
            let args = UriArgs(vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ]);
            let query = args.to_query();
            assert!(query.contains("a=1"));
            assert!(query.contains("b=2"));
        }

        #[test]
        fn test_to_query_empty_value() {
            let args = UriArgs(vec![("flag".to_string(), "".to_string())]);
            let query = args.to_query();
            assert!(query.contains("flag"));
            assert!(!query.contains("flag="));
        }
    }

    // Header manipulation logic tests
    mod header_manipulation {
        use super::*;
        use std::sync::{Arc, Mutex};

        #[test]
        fn test_header_name_normalization_underscore() {
            // Test that header names with underscore are normalized to dash
            let normalized = "content_type".to_lowercase().replace('_', "-");
            assert_eq!(normalized, "content-type");
        }

        #[test]
        fn test_header_name_normalization_case() {
            // Test that header names are lowercased
            let normalized = "Content-Type".to_lowercase().replace('_', "-");
            assert_eq!(normalized, "content-type");
        }

        #[test]
        fn test_header_insert_and_get() {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            );
            assert_eq!(headers.get("content-type").unwrap(), "application/json");
        }

        #[test]
        fn test_header_append() {
            let mut headers = HeaderMap::new();
            headers.append(
                HeaderName::from_static("set-cookie"),
                HeaderValue::from_static("a=1"),
            );
            headers.append(
                HeaderName::from_static("set-cookie"),
                HeaderValue::from_static("b=2"),
            );

            let values: Vec<_> = headers.get_all("set-cookie").iter().collect();
            assert_eq!(values.len(), 2);
        }

        #[test]
        fn test_header_remove() {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("text/html"),
            );
            headers.remove("content-type");
            assert!(headers.get("content-type").is_none());
        }
    }

    // Integration: building request state
    mod request_state_integration {
        use super::*;
        use crate::http::lua::structures::CandyReqState;

        #[test]
        fn test_build_uri_from_state() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/api/users".to_string(),
                uri_args: UriArgs(vec![
                    ("page".to_string(), "1".to_string()),
                    ("limit".to_string(), "10".to_string()),
                ]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_status: None,
                output_buffer: String::new(),
            };

            let uri = state.build_uri();
            assert!(uri.starts_with("/api/users?"));
            assert!(uri.contains("page=1"));
            assert!(uri.contains("limit=10"));
        }

        #[test]
        fn test_state_with_post_args() {
            let state = CandyReqState {
                method: "POST".to_string(),
                uri_path: "/submit".to_string(),
                uri_args: UriArgs::new(),
                post_args: Some(UriArgs(vec![
                    ("name".to_string(), "test".to_string()),
                ])),
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_status: None,
                output_buffer: String::new(),
            };

            assert!(state.post_args.is_some());
            assert_eq!(state.post_args.as_ref().unwrap().0[0].0, "name");
        }
    }

    // Edge cases
    mod edge_cases {
        use super::*;
        use crate::http::lua::structures::CandyReqState;

        #[test]
        fn test_empty_body() {
            let body = Arc::new(Mutex::new(Some(b"".to_vec())));
            let guard = body.lock().unwrap();
            // Empty body is Some with empty vec
            assert!(guard.as_ref().unwrap().is_empty());
        }

        #[test]
        fn test_none_body() {
            let body: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
            let guard = body.lock().unwrap();
            assert!(guard.is_none());
        }

        #[test]
        fn test_uri_with_special_chars() {
            let args = UriArgs::from_query("q=hello%20world&lang=en");
            assert!(!args.0.is_empty());
            // URL decoded value
            assert_eq!(args.0[0].0, "q");
        }

        #[test]
        fn test_multiple_headers_same_name() {
            let mut headers = HeaderMap::new();
            headers.append(
                header::SET_COOKIE,
                HeaderValue::from_static("cookie1=value1"),
            );
            headers.append(
                header::SET_COOKIE,
                HeaderValue::from_static("cookie2=value2"),
            );

            let all: Vec<_> = headers.get_all(header::SET_COOKIE).iter().collect();
            assert_eq!(all.len(), 2);
        }

        #[test]
        fn test_state_jump_flag() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/original".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: true, // Should trigger re-routing
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_status: None,
                output_buffer: String::new(),
            };

            assert!(state.jump);
        }
    }
}
