use bytes::Bytes;
use std::collections::HashMap;

/// HTTP/2 帧类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data,
    Headers,
    Priority,
    RstStream,
    Settings,
    PushPromise,
    Ping,
    GoAway,
    WindowUpdate,
    Continuation,
    Unknown(u8),
}

impl FrameType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => FrameType::Data,
            0x01 => FrameType::Headers,
            0x02 => FrameType::Priority,
            0x03 => FrameType::RstStream,
            0x04 => FrameType::Settings,
            0x05 => FrameType::PushPromise,
            0x06 => FrameType::Ping,
            0x07 => FrameType::GoAway,
            0x08 => FrameType::WindowUpdate,
            0x09 => FrameType::Continuation,
            _ => FrameType::Unknown(value),
        }
    }
}

/// HTTP/2 帧头（9 字节）
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// 帧长度（24 位）
    pub length: u32,
    /// 帧类型
    pub frame_type: FrameType,
    /// 帧标志
    pub flags: u8,
    /// 流标识符（31 位）
    pub stream_id: u32,
}

/// HTTP/2 帧解析错误
#[derive(Debug, thiserror::Error)]
pub enum H2Error {
    #[error("Incomplete frame")]
    Incomplete,

    #[error("Invalid stream ID")]
    InvalidStreamId,

    #[error("Invalid settings parameter")]
    InvalidSettings,

    #[error("Frame too large: {0} bytes")]
    TooLarge(u32),
}

/// HTTP/2 SETTINGS 参数
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsParameter {
    HeaderTableSize,
    EnablePush,
    MaxConcurrentStreams,
    InitialWindowSize,
    MaxFrameSize,
    MaxHeaderListSize,
    Unknown(u16),
}

impl SettingsParameter {
    pub fn from_u16(value: u16) -> Self {
        match value {
            0x01 => SettingsParameter::HeaderTableSize,
            0x02 => SettingsParameter::EnablePush,
            0x03 => SettingsParameter::MaxConcurrentStreams,
            0x04 => SettingsParameter::InitialWindowSize,
            0x05 => SettingsParameter::MaxFrameSize,
            0x06 => SettingsParameter::MaxHeaderListSize,
            _ => SettingsParameter::Unknown(value),
        }
    }
}

/// HTTP/2 SETTINGS 帧
#[derive(Debug, Clone)]
pub struct Settings {
    pub header_table_size: u32,
    pub enable_push: bool,
    pub max_concurrent_streams: Option<u32>,
    pub initial_window_size: u32,
    pub max_frame_size: u32,
    pub max_header_list_size: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            header_table_size: 4096,
            enable_push: true,
            max_concurrent_streams: None,
            initial_window_size: 65535,
            max_frame_size: 16384,
            max_header_list_size: None,
        }
    }
}

/// HTTP/2 连接前奏（magic string）
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// 解析帧头
pub fn parse_frame_header(data: &[u8]) -> Result<FrameHeader, H2Error> {
    if data.len() < 9 {
        return Err(H2Error::Incomplete);
    }

    // 长度：24 位大端序
    let length = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);

    // 最大帧大小 2^24 - 1 = 16MB
    if length > 16_777_215 {
        return Err(H2Error::TooLarge(length));
    }

    let frame_type = FrameType::from_u8(data[3]);
    let flags = data[4];

    // 流标识符：31 位（最高位保留）
    let stream_id = (((data[5] as u32) & 0x7F) << 24)
        | ((data[6] as u32) << 16)
        | ((data[7] as u32) << 8)
        | (data[8] as u32);

    Ok(FrameHeader {
        length,
        frame_type,
        flags,
        stream_id,
    })
}

/// 解析 SETTINGS 帧参数
pub fn parse_settings(data: &[u8]) -> Result<Settings, H2Error> {
    let mut settings = Settings::default();

    // SETTINGS 帧包含多个键值对，每个 6 字节
    if !data.len().is_multiple_of(6) {
        return Err(H2Error::InvalidSettings);
    }

    for chunk in data.chunks(6) {
        let id = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
        let value = ((chunk[2] as u32) << 24)
            | ((chunk[3] as u32) << 16)
            | ((chunk[4] as u32) << 8)
            | (chunk[5] as u32);

        match SettingsParameter::from_u16(id) {
            SettingsParameter::HeaderTableSize => {
                settings.header_table_size = value;
            }
            SettingsParameter::EnablePush => {
                settings.enable_push = value != 0;
            }
            SettingsParameter::MaxConcurrentStreams => {
                settings.max_concurrent_streams = Some(value);
            }
            SettingsParameter::InitialWindowSize => {
                settings.initial_window_size = value;
            }
            SettingsParameter::MaxFrameSize => {
                // 必须在 16384 到 16777215 之间
                if !(16384..=16_777_215).contains(&value) {
                    return Err(H2Error::InvalidSettings);
                }
                settings.max_frame_size = value;
            }
            SettingsParameter::MaxHeaderListSize => {
                settings.max_header_list_size = Some(value);
            }
            SettingsParameter::Unknown(_) => {
                // 忽略未知参数
            }
        }
    }

    Ok(settings)
}

/// 构建 SETTINGS ACK 帧
pub fn build_settings_ack() -> Bytes {
    // SETTINGS ACK 帧：长度 0，类型 0x04，标志 0x01，流 ID 0
    Bytes::from_static(&[0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00])
}

/// 构建初始 SETTINGS 帧
pub fn build_initial_settings() -> Bytes {
    let mut buf = Vec::new();

    // 帧头：长度 18（3 个参数），类型 SETTINGS，标志 0，流 ID 0
    buf.extend_from_slice(&[0x00, 0x00, 0x12, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // ENABLE_PUSH = 0（禁用服务器推送）
    buf.extend_from_slice(&[0x00, 0x02, 0x00, 0x00, 0x00, 0x00]);

    // MAX_FRAME_SIZE = 16384
    buf.extend_from_slice(&[0x00, 0x05, 0x00, 0x00, 0x40, 0x00]);

    // INITIAL_WINDOW_SIZE = 65535
    buf.extend_from_slice(&[0x00, 0x04, 0x00, 0x00, 0xFF, 0xFF]);

    Bytes::from(buf)
}

// ========== HTTP/2 Stream 管理 ==========

/// HTTP/2 流状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StreamState {
    Idle,
    ReservedLocal,
    ReservedRemote,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

/// HTTP/2 流
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Stream {
    /// 流标识符
    pub id: u32,
    /// 流状态
    pub state: StreamState,
    /// 发送窗口大小
    pub send_window: u32,
    /// 接收窗口大小
    pub recv_window: u32,
}

#[allow(dead_code)]
impl Stream {
    pub fn new(id: u32) -> Self {
        Stream {
            id,
            state: StreamState::Idle,
            send_window: 65535,
            recv_window: 65535,
        }
    }

    /// 转换到新状态
    pub fn transition(&mut self, new_state: StreamState) -> bool {
        // HTTP/2 流状态转换规则
        match (self.state, new_state) {
            // Idle 可以转换到任何状态
            (StreamState::Idle, _) => {
                self.state = new_state;
                true
            }
            // Open 可以转换到 HalfClosed 或 Closed
            (StreamState::Open, StreamState::HalfClosedLocal)
            | (StreamState::Open, StreamState::HalfClosedRemote)
            | (StreamState::Open, StreamState::Closed) => {
                self.state = new_state;
                true
            }
            // HalfClosed 可以转换到 Closed
            (StreamState::HalfClosedLocal, StreamState::Closed)
            | (StreamState::HalfClosedRemote, StreamState::Closed) => {
                self.state = new_state;
                true
            }
            // Reserved 可以转换到 HalfClosed 或 Closed
            (StreamState::ReservedLocal, StreamState::HalfClosedRemote)
            | (StreamState::ReservedLocal, StreamState::Closed)
            | (StreamState::ReservedRemote, StreamState::HalfClosedLocal)
            | (StreamState::ReservedRemote, StreamState::Closed) => {
                self.state = new_state;
                true
            }
            _ => false,
        }
    }

    /// 检查是否可以发送数据
    pub fn can_send(&self) -> bool {
        matches!(
            self.state,
            StreamState::Open | StreamState::HalfClosedRemote
        ) && self.send_window > 0
    }

    /// 检查是否可以接收数据
    pub fn can_receive(&self) -> bool {
        matches!(
            self.state,
            StreamState::Open | StreamState::HalfClosedLocal
        ) && self.recv_window > 0
    }
}

/// HTTP/2 连接管理器
#[derive(Debug)]
#[allow(dead_code)]
pub struct Connection {
    /// 服务端设置
    pub local_settings: Settings,
    /// 客户端设置
    pub peer_settings: Settings,
    /// 活跃流
    pub streams: std::collections::HashMap<u32, Stream>,
    /// 下一个可用的服务端流 ID（奇数）
    next_server_stream_id: u32,
    /// 连接级发送窗口大小
    pub connection_send_window: i64,
    /// 连接级接收窗口大小
    pub connection_recv_window: i64,
}

#[allow(dead_code)]
impl Connection {
    pub fn new() -> Self {
        Connection {
            local_settings: Settings::default(),
            peer_settings: Settings::default(),
            streams: std::collections::HashMap::new(),
            next_server_stream_id: 2, // 服务端流 ID 从 2 开始
            connection_send_window: 65535,
            connection_recv_window: 65535,
        }
    }

    /// 创建新的服务端流
    pub fn create_server_stream(&mut self) -> u32 {
        let id = self.next_server_stream_id;
        self.next_server_stream_id += 2;
        self.streams.insert(id, Stream::new(id));
        id
    }

    /// 接受客户端流
    pub fn accept_client_stream(&mut self, id: u32) -> Option<&mut Stream> {
        // 客户端流 ID 必须是奇数
        if id.is_multiple_of(2) || id == 0 {
            return None;
        }

        // 检查流 ID 是否递增
        let max_client_id = self
            .streams
            .keys()
            .filter(|&&k| k % 2 == 1)
            .max()
            .copied()
            .unwrap_or(0);

        if id <= max_client_id {
            return None;
        }

        self.streams.insert(id, Stream::new(id));
        self.streams.get_mut(&id)
    }

    /// 获取流
    pub fn get_stream(&self, id: u32) -> Option<&Stream> {
        self.streams.get(&id)
    }

    /// 获取可变流
    pub fn get_stream_mut(&mut self, id: u32) -> Option<&mut Stream> {
        self.streams.get_mut(&id)
    }

    /// 关闭流
    pub fn close_stream(&mut self, id: u32) {
        if let Some(stream) = self.streams.get_mut(&id) {
            stream.state = StreamState::Closed;
        }
    }

    /// 更新对端设置
    pub fn update_peer_settings(&mut self, settings: Settings) {
        self.peer_settings = settings;
    }

    /// 活跃流数量
    pub fn active_stream_count(&self) -> usize {
        self.streams
            .values()
            .filter(|s| s.state != StreamState::Closed)
            .count()
    }

    /// 更新连接级发送窗口
    pub fn update_connection_send_window(&mut self, increment: u32) -> Result<(), H2Error> {
        // 检查是否会溢出
        let new_window = self.connection_send_window + increment as i64;
        // HTTP/2 规范：窗口大小不能超过 2^31 - 1
        if new_window > (i32::MAX as i64) {
            return Err(H2Error::InvalidSettings);
        }
        self.connection_send_window = new_window;
        Ok(())
    }

    /// 更新流级发送窗口
    pub fn update_stream_send_window(&mut self, stream_id: u32, increment: u32) -> Result<(), H2Error> {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            let new_window = stream.send_window as i64 + increment as i64;
            if new_window > (i32::MAX as i64) {
                return Err(H2Error::InvalidSettings);
            }
            stream.send_window = new_window as u32;
        }
        Ok(())
    }

    /// 消耗连接级发送窗口
    pub fn consume_connection_send_window(&mut self, size: u32) {
        self.connection_send_window -= size as i64;
    }

    /// 消耗流级发送窗口
    pub fn consume_stream_send_window(&mut self, stream_id: u32, size: u32) {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.send_window = stream.send_window.saturating_sub(size);
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

// ========== HPACK 编解码 ==========

/// HPACK 编码器，用于压缩响应头
pub struct HpackEncoder {
    encoder: hpack::Encoder<'static>,
}

impl HpackEncoder {
    pub fn new(_max_table_size: u32) -> Self {
        HpackEncoder {
            encoder: hpack::Encoder::new(),
        }
    }

    /// 编码响应头
    pub fn encode_headers(&mut self, headers: &[(String, String)]) -> Bytes {
        let mut buf = Vec::new();
        for (name, value) in headers {
            let _ = self.encoder
                .encode_header_into((name.as_bytes(), value.as_bytes()), &mut buf);
        }
        Bytes::from(buf)
    }
}

impl Default for HpackEncoder {
    fn default() -> Self {
        Self::new(4096)
    }
}

/// HPACK 解码器，用于解压请求头
pub struct HpackDecoder {
    decoder: hpack::Decoder<'static>,
}

impl HpackDecoder {
    pub fn new(_max_table_size: u32) -> Self {
        HpackDecoder {
            decoder: hpack::Decoder::new(),
        }
    }

    /// 解码请求头
    pub fn decode_headers(&mut self, data: &[u8]) -> Result<HashMap<String, String>, H2Error> {
        let result = self.decoder.decode(data);

        match result {
            Ok(header_list) => {
                let mut headers = HashMap::new();
                for (name, value) in header_list {
                    let name_str = String::from_utf8_lossy(&name).to_string();
                    let value_str = String::from_utf8_lossy(&value).to_string();
                    headers.insert(name_str, value_str);
                }
                Ok(headers)
            }
            Err(_) => Err(H2Error::InvalidSettings),
        }
    }
}

impl Default for HpackDecoder {
    fn default() -> Self {
        Self::new(4096)
    }
}

// ========== HTTP/2 响应构建 ==========

/// 构建 HTTP/2 HEADERS 帧
pub fn build_headers_frame(stream_id: u32, headers: Bytes, end_stream: bool) -> Bytes {
    let length = headers.len() as u32;
    let flags = if end_stream { 0x05 } else { 0x04 }; // END_HEADERS | (END_STREAM if true)

    let mut buf = Vec::with_capacity(9 + headers.len());

    // 帧头：长度（3字节），类型 HEADERS(0x01)，标志，流ID
    buf.push((length >> 16) as u8);
    buf.push((length >> 8) as u8);
    buf.push(length as u8);
    buf.push(0x01); // HEADERS
    buf.push(flags);
    buf.push((stream_id >> 24) as u8 & 0x7F); // 最高位保留
    buf.push((stream_id >> 16) as u8);
    buf.push((stream_id >> 8) as u8);
    buf.push(stream_id as u8);

    // 帧体
    buf.extend_from_slice(&headers);

    Bytes::from(buf)
}

/// 构建 HTTP/2 DATA 帧
pub fn build_data_frame(stream_id: u32, data: &[u8], end_stream: bool) -> Bytes {
    let length = data.len() as u32;
    let flags = if end_stream { 0x01 } else { 0x00 }; // END_STREAM

    let mut buf = Vec::with_capacity(9 + data.len());

    // 帧头
    buf.push((length >> 16) as u8);
    buf.push((length >> 8) as u8);
    buf.push(length as u8);
    buf.push(0x00); // DATA
    buf.push(flags);
    buf.push((stream_id >> 24) as u8 & 0x7F);
    buf.push((stream_id >> 16) as u8);
    buf.push((stream_id >> 8) as u8);
    buf.push(stream_id as u8);

    // 帧体
    buf.extend_from_slice(data);

    Bytes::from(buf)
}

/// 构建 HTTP/2 RST_STREAM 帧
pub fn build_rst_stream(stream_id: u32, error_code: u32) -> Bytes {
    let mut buf = Vec::with_capacity(9 + 4);

    // 帧头：长度 4，类型 RST_STREAM(0x03)，标志 0
    buf.extend_from_slice(&[0x00, 0x00, 0x04, 0x03, 0x00]);
    buf.push((stream_id >> 24) as u8 & 0x7F);
    buf.push((stream_id >> 16) as u8);
    buf.push((stream_id >> 8) as u8);
    buf.push(stream_id as u8);

    // 错误码（4字节大端）
    buf.push((error_code >> 24) as u8);
    buf.push((error_code >> 16) as u8);
    buf.push((error_code >> 8) as u8);
    buf.push(error_code as u8);

    Bytes::from(buf)
}

/// 构建 HTTP/2 GOAWAY 帧
pub fn build_goaway(last_stream_id: u32, error_code: u32) -> Bytes {
    let mut buf = Vec::with_capacity(9 + 8);

    // 帧头：长度 8，类型 GOAWAY(0x07)，标志 0，流ID 0
    buf.extend_from_slice(&[0x00, 0x00, 0x08, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // 最后处理的流ID（4字节）
    buf.push((last_stream_id >> 24) as u8 & 0x7F);
    buf.push((last_stream_id >> 16) as u8);
    buf.push((last_stream_id >> 8) as u8);
    buf.push(last_stream_id as u8);

    // 错误码（4字节）
    buf.push((error_code >> 24) as u8);
    buf.push((error_code >> 16) as u8);
    buf.push((error_code >> 8) as u8);
    buf.push(error_code as u8);

    Bytes::from(buf)
}

/// 解析 WINDOW_UPDATE 帧
/// 返回窗口增量
pub fn parse_window_update(data: &[u8]) -> Result<u32, H2Error> {
    if data.len() < 4 {
        return Err(H2Error::Incomplete);
    }

    // 窗口增量（4字节大端序，最高位保留）
    let increment = (((data[0] as u32) & 0x7F) << 24)
        | ((data[1] as u32) << 16)
        | ((data[2] as u32) << 8)
        | (data[3] as u32);

    // 窗口增量不能为 0
    if increment == 0 {
        return Err(H2Error::InvalidSettings);
    }

    Ok(increment)
}

/// 构建 WINDOW_UPDATE 帧
#[allow(dead_code)]
pub fn build_window_update(stream_id: u32, increment: u32) -> Bytes {
    let mut buf = Vec::with_capacity(9 + 4);

    // 帧头：长度 4，类型 WINDOW_UPDATE(0x08)，标志 0
    buf.extend_from_slice(&[0x00, 0x00, 0x04, 0x08, 0x00]);
    buf.push((stream_id >> 24) as u8 & 0x7F);
    buf.push((stream_id >> 16) as u8);
    buf.push((stream_id >> 8) as u8);
    buf.push(stream_id as u8);

    // 窗口增量（4字节）
    buf.push((increment >> 24) as u8 & 0x7F);
    buf.push((increment >> 16) as u8);
    buf.push((increment >> 8) as u8);
    buf.push(increment as u8);

    Bytes::from(buf)
}

/// 构建 HTTP/2 PUSH_PROMISE 帧
#[allow(dead_code)]
pub fn build_push_promise(stream_id: u32, promised_stream_id: u32, headers: Bytes) -> Bytes {
    let length = 4 + headers.len() as u32; // 4 字节 promised stream id + headers
    let flags = 0x04; // END_HEADERS

    let mut buf = Vec::with_capacity(9 + 4 + headers.len());

    // 帧头
    buf.push((length >> 16) as u8);
    buf.push((length >> 8) as u8);
    buf.push(length as u8);
    buf.push(0x05); // PUSH_PROMISE
    buf.push(flags);
    buf.push((stream_id >> 24) as u8 & 0x7F);
    buf.push((stream_id >> 16) as u8);
    buf.push((stream_id >> 8) as u8);
    buf.push(stream_id as u8);

    // Promised Stream ID（4字节）
    buf.push((promised_stream_id >> 24) as u8 & 0x7F);
    buf.push((promised_stream_id >> 16) as u8);
    buf.push((promised_stream_id >> 8) as u8);
    buf.push(promised_stream_id as u8);

    // Headers
    buf.extend_from_slice(&headers);

    Bytes::from(buf)
}

/// HTTP/2 错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum H2ErrorCode {
    NoError = 0,
    ProtocolError = 1,
    InternalError = 2,
    FlowControlError = 3,
    SettingsTimeout = 4,
    StreamClosed = 5,
    FrameSizeError = 6,
    RefusedStream = 7,
    Cancel = 8,
    CompressionError = 9,
    ConnectError = 10,
    EnhanceYourCalm = 11,
    InadequateSecurity = 12,
    Http11Required = 13,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_from_u8() {
        assert_eq!(FrameType::from_u8(0x00), FrameType::Data);
        assert_eq!(FrameType::from_u8(0x01), FrameType::Headers);
        assert_eq!(FrameType::from_u8(0x04), FrameType::Settings);
        assert_eq!(FrameType::from_u8(0xFF), FrameType::Unknown(0xFF));
    }

    #[test]
    fn test_parse_frame_header_incomplete() {
        let data = [0x00, 0x00, 0x04];
        let result = parse_frame_header(&data);
        assert!(matches!(result, Err(H2Error::Incomplete)));
    }

    #[test]
    fn test_parse_frame_header_settings() {
        // SETTINGS 帧：长度 0，类型 0x04，标志 0x00，流 ID 0
        let data = [0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = parse_frame_header(&data).unwrap();
        assert_eq!(result.length, 0);
        assert_eq!(result.frame_type, FrameType::Settings);
        assert_eq!(result.flags, 0);
        assert_eq!(result.stream_id, 0);
    }

    #[test]
    fn test_parse_frame_header_data() {
        // DATA 帧：长度 5，类型 0x00，标志 0x01，流 ID 1
        let data = [0x00, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01];
        let result = parse_frame_header(&data).unwrap();
        assert_eq!(result.length, 5);
        assert_eq!(result.frame_type, FrameType::Data);
        assert_eq!(result.flags, 0x01);
        assert_eq!(result.stream_id, 1);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.header_table_size, 4096);
        assert!(settings.enable_push);
        assert_eq!(settings.initial_window_size, 65535);
        assert_eq!(settings.max_frame_size, 16384);
    }

    #[test]
    fn test_parse_settings_empty() {
        let data: &[u8] = &[];
        let result = parse_settings(data).unwrap();
        let default = Settings::default();
        assert_eq!(result.header_table_size, default.header_table_size);
    }

    #[test]
    fn test_parse_settings_enable_push() {
        // ENABLE_PUSH = 0
        let data = [0x00, 0x02, 0x00, 0x00, 0x00, 0x00];
        let result = parse_settings(&data).unwrap();
        assert!(!result.enable_push);
    }

    #[test]
    fn test_parse_settings_max_frame_size() {
        // MAX_FRAME_SIZE = 65536
        let data = [0x00, 0x05, 0x00, 0x01, 0x00, 0x00];
        let result = parse_settings(&data).unwrap();
        assert_eq!(result.max_frame_size, 65536);
    }

    #[test]
    fn test_build_settings_ack() {
        let frame = build_settings_ack();
        assert_eq!(frame.len(), 9);
        let header = parse_frame_header(&frame).unwrap();
        assert_eq!(header.frame_type, FrameType::Settings);
        assert_eq!(header.flags, 0x01); // ACK flag
    }

    #[test]
    fn test_build_initial_settings() {
        let frame = build_initial_settings();
        assert_eq!(frame.len(), 27); // 9 header + 18 payload
        let header = parse_frame_header(&frame).unwrap();
        assert_eq!(header.frame_type, FrameType::Settings);
        assert_eq!(header.length, 18);
    }

    #[test]
    fn test_connection_preface() {
        assert_eq!(
            CONNECTION_PREFACE,
            b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"
        );
        assert_eq!(CONNECTION_PREFACE.len(), 24);
    }

    #[test]
    fn test_settings_parameter_from_u16() {
        assert_eq!(
            SettingsParameter::from_u16(0x01),
            SettingsParameter::HeaderTableSize
        );
        assert_eq!(
            SettingsParameter::from_u16(0x05),
            SettingsParameter::MaxFrameSize
        );
        assert_eq!(SettingsParameter::from_u16(0xFF), SettingsParameter::Unknown(0xFF));
    }

    // ========== Stream 管理测试 ==========

    #[test]
    fn test_stream_new() {
        let stream = Stream::new(1);
        assert_eq!(stream.id, 1);
        assert_eq!(stream.state, StreamState::Idle);
        assert_eq!(stream.send_window, 65535);
        assert_eq!(stream.recv_window, 65535);
    }

    #[test]
    fn test_stream_transition_idle_to_open() {
        let mut stream = Stream::new(1);
        assert!(stream.transition(StreamState::Open));
        assert_eq!(stream.state, StreamState::Open);
    }

    #[test]
    fn test_stream_transition_open_to_half_closed() {
        let mut stream = Stream::new(1);
        stream.state = StreamState::Open;
        assert!(stream.transition(StreamState::HalfClosedLocal));
        assert_eq!(stream.state, StreamState::HalfClosedLocal);
    }

    #[test]
    fn test_stream_can_send() {
        let mut stream = Stream::new(1);
        assert!(!stream.can_send()); // Idle 状态不能发送

        stream.state = StreamState::Open;
        assert!(stream.can_send());

        stream.send_window = 0;
        assert!(!stream.can_send());
    }

    #[test]
    fn test_stream_can_receive() {
        let mut stream = Stream::new(1);
        assert!(!stream.can_receive()); // Idle 状态不能接收

        stream.state = StreamState::Open;
        assert!(stream.can_receive());

        stream.state = StreamState::HalfClosedLocal;
        assert!(stream.can_receive());
    }

    #[test]
    fn test_connection_new() {
        let conn = Connection::new();
        assert_eq!(conn.active_stream_count(), 0);
    }

    #[test]
    fn test_connection_create_server_stream() {
        let mut conn = Connection::new();
        let id = conn.create_server_stream();
        assert_eq!(id, 2); // 服务端流 ID 从 2 开始

        let id2 = conn.create_server_stream();
        assert_eq!(id2, 4); // 递增 2

        assert_eq!(conn.active_stream_count(), 2);
    }

    #[test]
    fn test_connection_accept_client_stream() {
        let mut conn = Connection::new();

        // 客户端流 ID 必须是奇数
        let stream = conn.accept_client_stream(1);
        assert!(stream.is_some());
        assert_eq!(stream.unwrap().id, 1);

        // 流 ID 必须递增
        let stream2 = conn.accept_client_stream(1);
        assert!(stream2.is_none()); // 重复 ID

        let stream3 = conn.accept_client_stream(3);
        assert!(stream3.is_some());

        // 偶数 ID 无效
        let stream4 = conn.accept_client_stream(4);
        assert!(stream4.is_none());
    }

    #[test]
    fn test_connection_close_stream() {
        let mut conn = Connection::new();
        conn.create_server_stream();
        conn.close_stream(2);

        let stream = conn.get_stream(2).unwrap();
        assert_eq!(stream.state, StreamState::Closed);
    }

    #[test]
    fn test_connection_update_peer_settings() {
        let mut conn = Connection::new();
        let mut settings = Settings::default();
        settings.enable_push = false;
        settings.max_frame_size = 65536;

        conn.update_peer_settings(settings);
        assert!(!conn.peer_settings.enable_push);
        assert_eq!(conn.peer_settings.max_frame_size, 65536);
    }

    #[test]
    fn test_connection_window() {
        let conn = Connection::new();
        assert_eq!(conn.connection_send_window, 65535);
        assert_eq!(conn.connection_recv_window, 65535);
    }

    #[test]
    fn test_connection_update_send_window() {
        let mut conn = Connection::new();
        conn.update_connection_send_window(1000).unwrap();
        assert_eq!(conn.connection_send_window, 66535);
    }

    #[test]
    fn test_connection_consume_send_window() {
        let mut conn = Connection::new();
        conn.consume_connection_send_window(1000);
        assert_eq!(conn.connection_send_window, 64535);
    }

    #[test]
    fn test_parse_window_update() {
        // 窗口增量 = 1000 (0x000003E8)
        let data = [0x00, 0x00, 0x03, 0xE8];
        let result = parse_window_update(&data).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_parse_window_update_incomplete() {
        let data = [0x00, 0x00];
        let result = parse_window_update(&data);
        assert!(matches!(result, Err(H2Error::Incomplete)));
    }

    #[test]
    fn test_parse_window_update_zero() {
        // 窗口增量 = 0，应该返回错误
        let data = [0x00, 0x00, 0x00, 0x00];
        let result = parse_window_update(&data);
        assert!(matches!(result, Err(H2Error::InvalidSettings)));
    }

    #[test]
    fn test_build_window_update() {
        let frame = build_window_update(0, 1000);
        assert_eq!(frame.len(), 13); // 9 header + 4 payload

        let header = parse_frame_header(&frame).unwrap();
        assert_eq!(header.frame_type, FrameType::WindowUpdate);
        assert_eq!(header.stream_id, 0);
        assert_eq!(header.length, 4);

        let increment = parse_window_update(&frame[9..]).unwrap();
        assert_eq!(increment, 1000);
    }

    #[test]
    fn test_build_push_promise() {
        let headers = Bytes::from_static(b"test-headers");
        let frame = build_push_promise(1, 2, headers.clone());

        let header = parse_frame_header(&frame).unwrap();
        assert_eq!(header.frame_type, FrameType::PushPromise);
        assert_eq!(header.stream_id, 1);
        assert_eq!(header.length, 4 + headers.len() as u32);

        // 验证 promised stream id
        let promised_id = (((frame[9] as u32) & 0x7F) << 24)
            | ((frame[10] as u32) << 16)
            | ((frame[11] as u32) << 8)
            | (frame[12] as u32);
        assert_eq!(promised_id, 2);
    }
}
