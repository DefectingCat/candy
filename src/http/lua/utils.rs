use anyhow;
use mlua::Error;

/// 将自 1970-01-01 以来的天数转换为年月日
pub fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    // 简化的日期计算算法
    let mut year = 1970;
    let mut remaining_days = days;

    // 计算年份
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // 每月天数
    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    // 计算月份和日期
    let mut month = 1u32;
    let mut day = 1u32;
    for &md in &month_days {
        if remaining_days < md {
            day = remaining_days as u32 + 1;
            break;
        }
        remaining_days -= md;
        month += 1;
    }

    (year, month, day)
}

/// 判断是否为闰年
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// URL 编码（简单实现，编码特殊字符）
pub fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// URL 解码
pub fn url_decode(s: &str) -> Result<String, Error> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() != 2 {
                    return Err(Error::external(anyhow::anyhow!("Invalid percent encoding")));
                }
                let byte = u8::from_str_radix(&hex, 16)
                    .map_err(|e| Error::external(anyhow::anyhow!("Invalid hex: {}", e)))?;
                result.push(byte as char);
            }
            _ => result.push(c),
        }
    }
    Ok(result)
}

/// 解析查询字符串为 key-value pairs
/// 无值参数 (如 ?foo&bar) 使用 "true" 作为值
/// 空键参数被丢弃
pub fn parse_query(query: &str) -> Vec<(String, String)> {
    if query.is_empty() {
        return Vec::new();
    }
    query
        .split('&')
        .filter_map(|pair| {
            if pair.is_empty() {
                return None;
            }
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let key = url_decode(k).ok()?;
            // 丢弃空键参数
            if key.is_empty() {
                return None;
            }
            let value = if v.is_empty() && !pair.contains('=') {
                // 无值参数 (如 ?foo) 使用 "true"
                String::new()
            } else {
                url_decode(v).ok()?
            };
            Some((key, value))
        })
        .collect()
}

/// 查询参数，保持顺序和重复键
#[derive(Clone, Debug, Default)]
pub struct UriArgs(pub Vec<(String, String)>);

impl UriArgs {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// 从查询字符串解析
    pub fn from_query(query: &str) -> Self {
        Self(parse_query(query))
    }

    /// 构建查询字符串
    pub fn to_query(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    url_encode(k)
                } else {
                    format!("{}={}", url_encode(k), url_encode(v))
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}
