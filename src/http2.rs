use bytes::Bytes;

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

    pub fn to_u8(&self) -> u8 {
        match self {
            FrameType::Data => 0x00,
            FrameType::Headers => 0x01,
            FrameType::Priority => 0x02,
            FrameType::RstStream => 0x03,
            FrameType::Settings => 0x04,
            FrameType::PushPromise => 0x05,
            FrameType::Ping => 0x06,
            FrameType::GoAway => 0x07,
            FrameType::WindowUpdate => 0x08,
            FrameType::Continuation => 0x09,
            FrameType::Unknown(v) => *v,
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

    #[error("Invalid frame length")]
    InvalidLength,

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

    pub fn to_u16(&self) -> u16 {
        match self {
            SettingsParameter::HeaderTableSize => 0x01,
            SettingsParameter::EnablePush => 0x02,
            SettingsParameter::MaxConcurrentStreams => 0x03,
            SettingsParameter::InitialWindowSize => 0x04,
            SettingsParameter::MaxFrameSize => 0x05,
            SettingsParameter::MaxHeaderListSize => 0x06,
            SettingsParameter::Unknown(v) => *v,
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
    if data.len() % 6 != 0 {
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
                if value < 16384 || value > 16_777_215 {
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
}
