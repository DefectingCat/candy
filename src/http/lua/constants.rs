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
