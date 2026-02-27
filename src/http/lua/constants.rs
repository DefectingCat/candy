/// HTTP 方法常量
pub const HTTP_GET: u16 = 0;
pub const HTTP_HEAD: u16 = 1;
pub const HTTP_PUT: u16 = 2;
pub const HTTP_POST: u16 = 3;
pub const HTTP_DELETE: u16 = 4;
pub const HTTP_OPTIONS: u16 = 5;
pub const HTTP_MKCOL: u16 = 6;
pub const HTTP_COPY: u16 = 7;
pub const HTTP_MOVE: u16 = 8;
pub const HTTP_PROPFIND: u16 = 9;
pub const HTTP_PROPPATCH: u16 = 10;
pub const HTTP_LOCK: u16 = 11;
pub const HTTP_UNLOCK: u16 = 12;
pub const HTTP_PATCH: u16 = 13;
pub const HTTP_TRACE: u16 = 14;

// HTTP 状态码常量 - 1xx
pub const HTTP_CONTINUE: u16 = 100;
pub const HTTP_SWITCHING_PROTOCOLS: u16 = 101;

// HTTP 状态码常量 - 2xx
pub const HTTP_OK: u16 = 200;
pub const HTTP_CREATED: u16 = 201;
pub const HTTP_ACCEPTED: u16 = 202;
pub const HTTP_NO_CONTENT: u16 = 204;
pub const HTTP_PARTIAL_CONTENT: u16 = 206;

// HTTP 状态码常量 - 3xx
pub const HTTP_SPECIAL_RESPONSE: u16 = 300;
pub const HTTP_MOVED_PERMANENTLY: u16 = 301;
pub const HTTP_MOVED_TEMPORARILY: u16 = 302;
pub const HTTP_SEE_OTHER: u16 = 303;
pub const HTTP_NOT_MODIFIED: u16 = 304;
pub const HTTP_TEMPORARY_REDIRECT: u16 = 307;

// HTTP 状态码常量 - 4xx
pub const HTTP_BAD_REQUEST: u16 = 400;
pub const HTTP_UNAUTHORIZED: u16 = 401;
pub const HTTP_PAYMENT_REQUIRED: u16 = 402;
pub const HTTP_FORBIDDEN: u16 = 403;
pub const HTTP_NOT_FOUND: u16 = 404;
pub const HTTP_NOT_ALLOWED: u16 = 405;
pub const HTTP_NOT_ACCEPTABLE: u16 = 406;
pub const HTTP_REQUEST_TIMEOUT: u16 = 408;
pub const HTTP_CONFLICT: u16 = 409;
pub const HTTP_GONE: u16 = 410;
pub const HTTP_UPGRADE_REQUIRED: u16 = 426;
pub const HTTP_TOO_MANY_REQUESTS: u16 = 429;
pub const HTTP_CLOSE: u16 = 444;
pub const HTTP_ILLEGAL: u16 = 451;

// HTTP 状态码常量 - 5xx
pub const HTTP_INTERNAL_SERVER_ERROR: u16 = 500;
pub const HTTP_METHOD_NOT_IMPLEMENTED: u16 = 501;
pub const HTTP_BAD_GATEWAY: u16 = 502;
pub const HTTP_SERVICE_UNAVAILABLE: u16 = 503;
pub const HTTP_GATEWAY_TIMEOUT: u16 = 504;
pub const HTTP_VERSION_NOT_SUPPORTED: u16 = 505;
pub const HTTP_INSUFFICIENT_STORAGE: u16 = 507;

#[cfg(test)]
mod tests {
    use super::*;

    // HTTP 方法常量测试
    mod http_methods {
        use super::*;

        #[test]
        fn test_http_method_values() {
            assert_eq!(HTTP_GET, 0);
            assert_eq!(HTTP_HEAD, 1);
            assert_eq!(HTTP_PUT, 2);
            assert_eq!(HTTP_POST, 3);
            assert_eq!(HTTP_DELETE, 4);
            assert_eq!(HTTP_OPTIONS, 5);
            assert_eq!(HTTP_MKCOL, 6);
            assert_eq!(HTTP_COPY, 7);
            assert_eq!(HTTP_MOVE, 8);
            assert_eq!(HTTP_PROPFIND, 9);
            assert_eq!(HTTP_PROPPATCH, 10);
            assert_eq!(HTTP_LOCK, 11);
            assert_eq!(HTTP_UNLOCK, 12);
            assert_eq!(HTTP_PATCH, 13);
            assert_eq!(HTTP_TRACE, 14);
        }

        #[test]
        fn test_http_methods_are_unique() {
            let methods = [
                HTTP_GET,
                HTTP_HEAD,
                HTTP_PUT,
                HTTP_POST,
                HTTP_DELETE,
                HTTP_OPTIONS,
                HTTP_MKCOL,
                HTTP_COPY,
                HTTP_MOVE,
                HTTP_PROPFIND,
                HTTP_PROPPATCH,
                HTTP_LOCK,
                HTTP_UNLOCK,
                HTTP_PATCH,
                HTTP_TRACE,
            ];
            let mut sorted = methods.to_vec();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), methods.len());
        }
    }

    // 1xx 信息响应测试
    mod informational {
        use super::*;

        #[test]
        fn test_1xx_status_codes() {
            assert_eq!(HTTP_CONTINUE, 100);
            assert_eq!(HTTP_SWITCHING_PROTOCOLS, 101);
        }

        #[test]
        fn test_1xx_range() {
            assert!(HTTP_CONTINUE >= 100 && HTTP_CONTINUE < 200);
            assert!(HTTP_SWITCHING_PROTOCOLS >= 100 && HTTP_SWITCHING_PROTOCOLS < 200);
        }
    }

    // 2xx 成功响应测试
    mod successful {
        use super::*;

        #[test]
        fn test_2xx_status_codes() {
            assert_eq!(HTTP_OK, 200);
            assert_eq!(HTTP_CREATED, 201);
            assert_eq!(HTTP_ACCEPTED, 202);
            assert_eq!(HTTP_NO_CONTENT, 204);
            assert_eq!(HTTP_PARTIAL_CONTENT, 206);
        }

        #[test]
        fn test_2xx_range() {
            assert!(HTTP_OK >= 200 && HTTP_OK < 300);
            assert!(HTTP_CREATED >= 200 && HTTP_CREATED < 300);
            assert!(HTTP_ACCEPTED >= 200 && HTTP_ACCEPTED < 300);
            assert!(HTTP_NO_CONTENT >= 200 && HTTP_NO_CONTENT < 300);
            assert!(HTTP_PARTIAL_CONTENT >= 200 && HTTP_PARTIAL_CONTENT < 300);
        }

        #[test]
        fn test_no_205_in_constants() {
            // HTTP 205 Reset Content 不在常用常量中
            let all_2xx = [
                HTTP_OK,
                HTTP_CREATED,
                HTTP_ACCEPTED,
                HTTP_NO_CONTENT,
                HTTP_PARTIAL_CONTENT,
            ];
            assert!(!all_2xx.contains(&205));
        }
    }

    // 3xx 重定向响应测试
    mod redirection {
        use super::*;

        #[test]
        fn test_3xx_status_codes() {
            assert_eq!(HTTP_SPECIAL_RESPONSE, 300);
            assert_eq!(HTTP_MOVED_PERMANENTLY, 301);
            assert_eq!(HTTP_MOVED_TEMPORARILY, 302);
            assert_eq!(HTTP_SEE_OTHER, 303);
            assert_eq!(HTTP_NOT_MODIFIED, 304);
            assert_eq!(HTTP_TEMPORARY_REDIRECT, 307);
        }

        #[test]
        fn test_3xx_range() {
            assert!(HTTP_SPECIAL_RESPONSE >= 300 && HTTP_SPECIAL_RESPONSE < 400);
            assert!(HTTP_MOVED_PERMANENTLY >= 300 && HTTP_MOVED_PERMANENTLY < 400);
            assert!(HTTP_MOVED_TEMPORARILY >= 300 && HTTP_MOVED_TEMPORARILY < 400);
            assert!(HTTP_SEE_OTHER >= 300 && HTTP_SEE_OTHER < 400);
            assert!(HTTP_NOT_MODIFIED >= 300 && HTTP_NOT_MODIFIED < 400);
            assert!(HTTP_TEMPORARY_REDIRECT >= 300 && HTTP_TEMPORARY_REDIRECT < 400);
        }

        #[test]
        fn test_no_304_in_constants() {
            // HTTP 304 Not Modified 已存在
            let all_3xx = [
                HTTP_SPECIAL_RESPONSE,
                HTTP_MOVED_PERMANENTLY,
                HTTP_MOVED_TEMPORARILY,
                HTTP_SEE_OTHER,
                HTTP_NOT_MODIFIED,
                HTTP_TEMPORARY_REDIRECT,
            ];
            assert!(all_3xx.contains(&304));
        }
    }

    // 4xx 客户端错误响应测试
    mod client_error {
        use super::*;

        #[test]
        fn test_4xx_status_codes() {
            assert_eq!(HTTP_BAD_REQUEST, 400);
            assert_eq!(HTTP_UNAUTHORIZED, 401);
            assert_eq!(HTTP_PAYMENT_REQUIRED, 402);
            assert_eq!(HTTP_FORBIDDEN, 403);
            assert_eq!(HTTP_NOT_FOUND, 404);
            assert_eq!(HTTP_NOT_ALLOWED, 405);
            assert_eq!(HTTP_NOT_ACCEPTABLE, 406);
            assert_eq!(HTTP_REQUEST_TIMEOUT, 408);
            assert_eq!(HTTP_CONFLICT, 409);
            assert_eq!(HTTP_GONE, 410);
            assert_eq!(HTTP_UPGRADE_REQUIRED, 426);
            assert_eq!(HTTP_TOO_MANY_REQUESTS, 429);
            assert_eq!(HTTP_CLOSE, 444);
            assert_eq!(HTTP_ILLEGAL, 451);
        }

        #[test]
        fn test_4xx_range() {
            assert!(HTTP_BAD_REQUEST >= 400 && HTTP_BAD_REQUEST < 500);
            assert!(HTTP_UNAUTHORIZED >= 400 && HTTP_UNAUTHORIZED < 500);
            assert!(HTTP_FORBIDDEN >= 400 && HTTP_FORBIDDEN < 500);
            assert!(HTTP_NOT_FOUND >= 400 && HTTP_NOT_FOUND < 500);
            assert!(HTTP_NOT_ALLOWED >= 400 && HTTP_NOT_ALLOWED < 500);
            assert!(HTTP_TOO_MANY_REQUESTS >= 400 && HTTP_TOO_MANY_REQUESTS < 500);
        }

        #[test]
        fn test_4xx_are_unique() {
            let codes = [
                HTTP_BAD_REQUEST,
                HTTP_UNAUTHORIZED,
                HTTP_PAYMENT_REQUIRED,
                HTTP_FORBIDDEN,
                HTTP_NOT_FOUND,
                HTTP_NOT_ALLOWED,
                HTTP_NOT_ACCEPTABLE,
                HTTP_REQUEST_TIMEOUT,
                HTTP_CONFLICT,
                HTTP_GONE,
                HTTP_UPGRADE_REQUIRED,
                HTTP_TOO_MANY_REQUESTS,
                HTTP_CLOSE,
                HTTP_ILLEGAL,
            ];
            let mut sorted = codes.to_vec();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), codes.len());
        }
    }

    // 5xx 服务器错误响应测试
    mod server_error {
        use super::*;

        #[test]
        fn test_5xx_status_codes() {
            assert_eq!(HTTP_INTERNAL_SERVER_ERROR, 500);
            assert_eq!(HTTP_METHOD_NOT_IMPLEMENTED, 501);
            assert_eq!(HTTP_BAD_GATEWAY, 502);
            assert_eq!(HTTP_SERVICE_UNAVAILABLE, 503);
            assert_eq!(HTTP_GATEWAY_TIMEOUT, 504);
            assert_eq!(HTTP_VERSION_NOT_SUPPORTED, 505);
            assert_eq!(HTTP_INSUFFICIENT_STORAGE, 507);
        }

        #[test]
        fn test_5xx_range() {
            assert!(HTTP_INTERNAL_SERVER_ERROR >= 500 && HTTP_INTERNAL_SERVER_ERROR < 600);
            assert!(HTTP_METHOD_NOT_IMPLEMENTED >= 500 && HTTP_METHOD_NOT_IMPLEMENTED < 600);
            assert!(HTTP_BAD_GATEWAY >= 500 && HTTP_BAD_GATEWAY < 600);
            assert!(HTTP_SERVICE_UNAVAILABLE >= 500 && HTTP_SERVICE_UNAVAILABLE < 600);
            assert!(HTTP_GATEWAY_TIMEOUT >= 500 && HTTP_GATEWAY_TIMEOUT < 600);
            assert!(HTTP_VERSION_NOT_SUPPORTED >= 500 && HTTP_VERSION_NOT_SUPPORTED < 600);
            assert!(HTTP_INSUFFICIENT_STORAGE >= 500 && HTTP_INSUFFICIENT_STORAGE < 600);
        }

        #[test]
        fn test_5xx_are_unique() {
            let codes = [
                HTTP_INTERNAL_SERVER_ERROR,
                HTTP_METHOD_NOT_IMPLEMENTED,
                HTTP_BAD_GATEWAY,
                HTTP_SERVICE_UNAVAILABLE,
                HTTP_GATEWAY_TIMEOUT,
                HTTP_VERSION_NOT_SUPPORTED,
                HTTP_INSUFFICIENT_STORAGE,
            ];
            let mut sorted = codes.to_vec();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), codes.len());
        }
    }

    // 整体常量一致性测试
    mod consistency {
        use super::*;

        #[test]
        fn test_all_status_codes_unique() {
            let all_codes = [
                // 1xx
                HTTP_CONTINUE,
                HTTP_SWITCHING_PROTOCOLS,
                // 2xx
                HTTP_OK,
                HTTP_CREATED,
                HTTP_ACCEPTED,
                HTTP_NO_CONTENT,
                HTTP_PARTIAL_CONTENT,
                // 3xx
                HTTP_SPECIAL_RESPONSE,
                HTTP_MOVED_PERMANENTLY,
                HTTP_MOVED_TEMPORARILY,
                HTTP_SEE_OTHER,
                HTTP_NOT_MODIFIED,
                HTTP_TEMPORARY_REDIRECT,
                // 4xx
                HTTP_BAD_REQUEST,
                HTTP_UNAUTHORIZED,
                HTTP_PAYMENT_REQUIRED,
                HTTP_FORBIDDEN,
                HTTP_NOT_FOUND,
                HTTP_NOT_ALLOWED,
                HTTP_NOT_ACCEPTABLE,
                HTTP_REQUEST_TIMEOUT,
                HTTP_CONFLICT,
                HTTP_GONE,
                HTTP_UPGRADE_REQUIRED,
                HTTP_TOO_MANY_REQUESTS,
                HTTP_CLOSE,
                HTTP_ILLEGAL,
                // 5xx
                HTTP_INTERNAL_SERVER_ERROR,
                HTTP_METHOD_NOT_IMPLEMENTED,
                HTTP_BAD_GATEWAY,
                HTTP_SERVICE_UNAVAILABLE,
                HTTP_GATEWAY_TIMEOUT,
                HTTP_VERSION_NOT_SUPPORTED,
                HTTP_INSUFFICIENT_STORAGE,
            ];
            let mut sorted = all_codes.to_vec();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), all_codes.len());
        }

        #[test]
        fn test_status_code_ranges() {
            // 验证所有状态码在有效范围内
            let all_codes = [
                HTTP_CONTINUE,
                HTTP_SWITCHING_PROTOCOLS,
                HTTP_OK,
                HTTP_CREATED,
                HTTP_ACCEPTED,
                HTTP_NO_CONTENT,
                HTTP_PARTIAL_CONTENT,
                HTTP_SPECIAL_RESPONSE,
                HTTP_MOVED_PERMANENTLY,
                HTTP_MOVED_TEMPORARILY,
                HTTP_SEE_OTHER,
                HTTP_NOT_MODIFIED,
                HTTP_TEMPORARY_REDIRECT,
                HTTP_BAD_REQUEST,
                HTTP_UNAUTHORIZED,
                HTTP_PAYMENT_REQUIRED,
                HTTP_FORBIDDEN,
                HTTP_NOT_FOUND,
                HTTP_NOT_ALLOWED,
                HTTP_NOT_ACCEPTABLE,
                HTTP_REQUEST_TIMEOUT,
                HTTP_CONFLICT,
                HTTP_GONE,
                HTTP_UPGRADE_REQUIRED,
                HTTP_TOO_MANY_REQUESTS,
                HTTP_CLOSE,
                HTTP_ILLEGAL,
                HTTP_INTERNAL_SERVER_ERROR,
                HTTP_METHOD_NOT_IMPLEMENTED,
                HTTP_BAD_GATEWAY,
                HTTP_SERVICE_UNAVAILABLE,
                HTTP_GATEWAY_TIMEOUT,
                HTTP_VERSION_NOT_SUPPORTED,
                HTTP_INSUFFICIENT_STORAGE,
            ];

            for code in all_codes {
                assert!(
                    (100..=599).contains(&code),
                    "Status code {} is out of valid HTTP range (100-599)",
                    code
                );
            }
        }

        #[test]
        fn test_common_status_codes_exist() {
            // 验证常见状态码存在
            let common = [
                (HTTP_OK, 200, "OK"),
                (HTTP_CREATED, 201, "Created"),
                (HTTP_NO_CONTENT, 204, "No Content"),
                (HTTP_MOVED_PERMANENTLY, 301, "Moved Permanently"),
                (HTTP_NOT_MODIFIED, 304, "Not Modified"),
                (HTTP_BAD_REQUEST, 400, "Bad Request"),
                (HTTP_UNAUTHORIZED, 401, "Unauthorized"),
                (HTTP_FORBIDDEN, 403, "Forbidden"),
                (HTTP_NOT_FOUND, 404, "Not Found"),
                (HTTP_INTERNAL_SERVER_ERROR, 500, "Internal Server Error"),
                (HTTP_NOT_ALLOWED, 405, "Method Not Allowed"),
                (HTTP_SERVICE_UNAVAILABLE, 503, "Service Unavailable"),
            ];

            for (const_val, expected_val, name) in common {
                assert_eq!(
                    const_val, expected_val,
                    "Constant for {} should be {}",
                    name, expected_val
                );
            }
        }
    }
}
