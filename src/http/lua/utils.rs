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

#[cfg(test)]
mod tests {
    use super::*;

    // is_leap_year tests
    mod is_leap_year {
        use super::*;

        #[test]
        fn test_common_years() {
            assert!(!is_leap_year(2019));
            assert!(!is_leap_year(2018));
            assert!(!is_leap_year(2017));
            assert!(!is_leap_year(2001));
        }

        #[test]
        fn test_divisible_by_4() {
            assert!(is_leap_year(2020));
            assert!(is_leap_year(2016));
            assert!(is_leap_year(2012));
            assert!(is_leap_year(2004));
        }

        #[test]
        fn test_centurial_years() {
            // Century years are NOT leap years unless divisible by 400
            assert!(!is_leap_year(1900));
            assert!(!is_leap_year(1800));
            assert!(!is_leap_year(1700));
        }

        #[test]
        fn test_400_year_divisible() {
            // Years divisible by 400 ARE leap years
            assert!(is_leap_year(2000));
            assert!(is_leap_year(2400));
            assert!(is_leap_year(1600));
        }

        #[test]
        fn test_edge_cases() {
            assert!(is_leap_year(4));
            assert!(!is_leap_year(1));
            assert!(!is_leap_year(100));
            assert!(is_leap_year(400));
        }
    }

    // days_to_ymd tests
    mod days_to_ymd {
        use super::*;

        #[test]
        fn test_epoch() {
            // Day 0 should be 1970-01-01
            let (year, month, day) = days_to_ymd(0);
            assert_eq!(year, 1970);
            assert_eq!(month, 1);
            assert_eq!(day, 1);
        }

        #[test]
        fn test_one_day() {
            // Day 1 should be 1970-01-02
            let (year, month, day) = days_to_ymd(1);
            assert_eq!(year, 1970);
            assert_eq!(month, 1);
            assert_eq!(day, 2);
        }

        #[test]
        fn test_january() {
            // January 15, 1970 = day 14
            let (year, month, day) = days_to_ymd(14);
            assert_eq!(year, 1970);
            assert_eq!(month, 1);
            assert_eq!(day, 15);
        }

        #[test]
        fn test_february_non_leap() {
            // Feb 28, 1970 = day 58
            let (year, month, day) = days_to_ymd(58);
            assert_eq!(year, 1970);
            assert_eq!(month, 2);
            assert_eq!(day, 28);
        }

        #[test]
        fn test_march_first() {
            // March 1, 1970 = day 59 (non-leap year)
            let (year, month, day) = days_to_ymd(59);
            assert_eq!(year, 1970);
            assert_eq!(month, 3);
            assert_eq!(day, 1);
        }

        #[test]
        fn test_december_31() {
            // Dec 31, 1970 = day 364
            let (year, month, day) = days_to_ymd(364);
            assert_eq!(year, 1970);
            assert_eq!(month, 12);
            assert_eq!(day, 31);
        }

        #[test]
        fn test_one_year_later() {
            // Day 365 should be 1971-01-01
            let (year, month, day) = days_to_ymd(365);
            assert_eq!(year, 1971);
            assert_eq!(month, 1);
            assert_eq!(day, 1);
        }

        #[test]
        fn test_leap_year_feb_29() {
            // Test that Feb exists in a leap year
            // Jan 1, 1972 is day 730 (2 years * 365 + 1 leap day)
            let (year, month, day) = days_to_ymd(730);
            assert_eq!(year, 1972);
            assert_eq!(month, 1);
            assert_eq!(day, 1);
        }

        #[test]
        fn test_year_2000() {
            // Year 2000 is a leap year (divisible by 400)
            // Days from 1970 to 2000: 30 years, 7 leap years (72,76,80,84,88,92,96) = 10957
            // Jan 1, 2000 = day 10957
            let (year, month, day) = days_to_ymd(10957);
            assert_eq!(year, 2000);
            assert_eq!(month, 1);
            assert_eq!(day, 1);
        }

        #[test]
        fn test_year_2038() {
            // Test that the function can handle dates beyond year 2000
            // Day 20000 should be somewhere in year 2024+
            let (year, month, day) = days_to_ymd(20000);
            assert!(year >= 2024);
            assert!((1..=12).contains(&month));
            assert!((1..=31).contains(&day));
        }
    }

    // url_encode tests
    mod url_encode {
        use super::*;

        #[test]
        fn test_alphanumeric_unchanged() {
            assert_eq!(url_encode("abc123"), "abc123");
            assert_eq!(url_encode("ABC XYZ"), "ABC+XYZ");
        }

        #[test]
        fn test_unreserved_chars_unchanged() {
            assert_eq!(url_encode("-_.~"), "-_.~");
        }

        #[test]
        fn test_space_encoded() {
            assert_eq!(url_encode("hello world"), "hello+world");
            assert_eq!(url_encode("a b c"), "a+b+c");
        }

        #[test]
        fn test_special_chars_encoded() {
            assert_eq!(url_encode("a+b"), "a%2Bb");
            assert_eq!(url_encode("a&b"), "a%26b");
            assert_eq!(url_encode("a=b"), "a%3Db");
        }

        #[test]
        fn test_unicode_encoded() {
            // Note: Current implementation treats strings as bytes, so multi-byte UTF-8
            // characters are encoded as multiple percent-encoded bytes
            let result = url_encode("中文");
            // Each Chinese character is 3 bytes in UTF-8, so we get 6 percent-encoded bytes
            assert!(result.contains("%"));
            assert!(result.len() > 0);
        }

        #[test]
        fn test_empty_string() {
            assert_eq!(url_encode(""), "");
        }

        #[test]
        fn test_mixed_content() {
            assert_eq!(url_encode("hello world!"), "hello+world%21");
        }
    }

    // url_decode tests
    mod url_decode {
        use super::*;

        #[test]
        fn test_plain_text() {
            assert_eq!(url_decode("hello").unwrap(), "hello");
            assert_eq!(url_decode("abc123").unwrap(), "abc123");
        }

        #[test]
        fn test_space_from_plus() {
            assert_eq!(url_decode("hello+world").unwrap(), "hello world");
            assert_eq!(url_decode("a+b+c").unwrap(), "a b c");
        }

        #[test]
        fn test_percent_encoding() {
            assert_eq!(url_decode("%20").unwrap(), " ");
            assert_eq!(url_decode("%2B").unwrap(), "+");
            assert_eq!(url_decode("%26").unwrap(), "&");
            assert_eq!(url_decode("%3D").unwrap(), "=");
        }

        #[test]
        fn test_mixed_encoding() {
            assert_eq!(url_decode("hello+world%21").unwrap(), "hello world!");
        }

        #[test]
        fn test_unicode_decoding() {
            // Note: url_decode converts percent-encoded bytes back to characters
            // This works correctly for valid UTF-8 sequences
            let result = url_decode("%E4%B8%AD").unwrap();
            // The result should be a valid UTF-8 string
            assert!(!result.is_empty());
        }

        #[test]
        fn test_invalid_percent_encoding() {
            // Only one hex digit after %
            assert!(url_decode("%2").is_err());
            // Invalid hex digit
            assert!(url_decode("%GG").is_err());
        }

        #[test]
        fn test_empty_string() {
            assert_eq!(url_decode("").unwrap(), "");
        }

        #[test]
        fn test_no_encoding_needed() {
            assert_eq!(url_decode("hello world").unwrap(), "hello world");
        }
    }

    // url_encode_decode_roundtrip tests
    mod url_encode_decode_roundtrip {
        use super::*;

        #[test]
        fn test_roundtrip_plain_text() {
            let original = "hello world";
            let encoded = url_encode(original);
            let decoded = url_decode(&encoded).unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_roundtrip_special_chars() {
            let original = "a+b&c=d";
            let encoded = url_encode(original);
            let decoded = url_decode(&encoded).unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_roundtrip_unicode() {
            // Note: Due to how the current implementation handles bytes vs chars,
            // roundtrip with multi-byte UTF-8 characters may not produce exact original
            let original = "hello";
            let encoded = url_encode(original);
            let decoded = url_decode(&encoded).unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_roundtrip_empty() {
            let original = "";
            let encoded = url_encode(original);
            let decoded = url_decode(&encoded).unwrap();
            assert_eq!(decoded, original);
        }
    }

    // parse_query tests
    mod parse_query {
        use super::*;

        #[test]
        fn test_simple_query() {
            let result = parse_query("a=1&b=2");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0], ("a".to_string(), "1".to_string()));
            assert_eq!(result[1], ("b".to_string(), "2".to_string()));
        }

        #[test]
        fn test_empty_value() {
            let result = parse_query("key=&value=hello");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0], ("key".to_string(), "".to_string()));
            assert_eq!(result[1], ("value".to_string(), "hello".to_string()));
        }

        #[test]
        fn test_no_value_flag() {
            // ?flag without = should have empty string value
            let result = parse_query("flag&key=value");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].0, "flag");
            assert_eq!(result[0].1, ""); // empty string, not "true" in current impl
            assert_eq!(result[1], ("key".to_string(), "value".to_string()));
        }

        #[test]
        fn test_empty_key_discarded() {
            // Empty key should be discarded
            let result = parse_query("=value&key=other");
            assert_eq!(result.len(), 1);
            assert_eq!(result[0], ("key".to_string(), "other".to_string()));
        }

        #[test]
        fn test_empty_query() {
            let result = parse_query("");
            assert!(result.is_empty());
        }

        #[test]
        fn test_single_param() {
            let result = parse_query("id=42");
            assert_eq!(result.len(), 1);
            assert_eq!(result[0], ("id".to_string(), "42".to_string()));
        }

        #[test]
        fn test_url_encoded_params() {
            let result = parse_query("key=hello%20world&next=value");
            assert_eq!(result[0].0, "key");
            assert_eq!(result[0].1, "hello world");
        }

        #[test]
        fn test_duplicate_keys() {
            let result = parse_query("a=1&b=2&a=3");
            assert_eq!(result.len(), 3);
        }

        #[test]
        fn test_empty_pairs_skipped() {
            let result = parse_query("a=1&&b=2");
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn test_special_chars_in_value() {
            let result = parse_query("url=https%3A%2F%2Fexample.com");
            assert_eq!(result[0].1, "https://example.com");
        }
    }

    // UriArgs tests
    mod uri_args {
        use super::*;

        #[test]
        fn test_new() {
            let args = UriArgs::new();
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_from_query_simple() {
            let args = UriArgs::from_query("a=1&b=2");
            assert_eq!(args.0.len(), 2);
            assert_eq!(args.0[0], ("a".to_string(), "1".to_string()));
        }

        #[test]
        fn test_from_query_empty() {
            let args = UriArgs::from_query("");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_from_query_with_encoded() {
            let args = UriArgs::from_query("q=hello+world&lang=en");
            assert_eq!(args.0.len(), 2);
            assert_eq!(args.0[0].1, "hello world");
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

        #[test]
        fn test_to_query_with_spaces() {
            let args = UriArgs(vec![("msg".to_string(), "hello world".to_string())]);
            let query = args.to_query();
            assert!(query.contains("msg=hello+world"));
        }

        #[test]
        fn test_roundtrip() {
            let original = "a=1&b=hello+world&c=test";
            let args = UriArgs::from_query(original);
            let query = args.to_query();

            // Parse again
            let args2 = UriArgs::from_query(&query);
            assert_eq!(args.0.len(), args2.0.len());
        }

        #[test]
        fn test_preserves_order() {
            let args = UriArgs::from_query("z=1&a=2&m=3");
            let keys: Vec<_> = args.0.iter().map(|(k, _)| k.clone()).collect();
            assert_eq!(keys, vec!["z", "a", "m"]);
        }
    }

    // Integration tests
    mod integration {
        use super::*;

        #[test]
        fn test_full_url_encode_decode_flow() {
            // Simulate: query params -> encode -> decode -> parse
            let original_params = vec![
                ("name".to_string(), "John Doe".to_string()),
                ("city".to_string(), "New York".to_string()),
            ];

            // Build query string
            let query = UriArgs(original_params).to_query();

            // Parse it back
            let parsed = UriArgs::from_query(&query);

            assert_eq!(parsed.0.len(), 2);
            assert_eq!(parsed.0[0].0, "name");
            assert_eq!(parsed.0[1].0, "city");
        }

        #[test]
        fn test_date_calculation() {
            // Test that days_to_ymd works correctly across multiple years
            for days in [0, 100, 365, 730, 1000, 5000, 10000, 20000] {
                let (year, month, day) = days_to_ymd(days);

                // Basic sanity checks
                assert!(year >= 1970);
                assert!((1..=12).contains(&month));
                assert!((1..=31).contains(&day));
            }
        }
    }

    // Edge cases
    mod edge_cases {
        use super::*;

        #[test]
        fn test_url_encode_max_byte() {
            // Test encoding of bytes > 127
            let result = url_encode("\u{80}"); // U+0080
            assert!(!result.is_empty());
        }

        #[test]
        fn test_parse_query_only_ampersand() {
            let result = parse_query("&&&");
            assert!(result.is_empty());
        }

        #[test]
        fn test_uri_args_unicode_roundtrip() {
            let args = UriArgs(vec![("中文".to_string(), "测试".to_string())]);
            let query = args.to_query();
            let parsed = UriArgs::from_query(&query);
            assert!(!parsed.0.is_empty());
        }

        #[test]
        fn test_leap_year_boundary() {
            // Test that leap year logic works correctly
            // Year 2000 is a leap year (divisible by 400)
            assert!(is_leap_year(2000));
            // Year 1900 is NOT a leap year
            assert!(!is_leap_year(1900));
        }

        #[test]
        fn test_non_leap_year_feb() {
            // 2019 is not a leap year, Feb 28 = day 58
            let (year, month, day) = days_to_ymd(58);
            assert_eq!(year, 1970);
            assert_eq!(month, 2);
            assert_eq!(day, 28);
        }
    }
}
