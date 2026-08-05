//! Binary framed adapter transport.
//!
//! Frame layout (little-endian):
//! `MAGIC("ELCX") | KIND(u8) | ID(u64) | JSON_LEN(u32) | BIN_LEN(u32) | JSON | BIN`

use std::io::{Read, Write};

use crate::{
    AdapterEvent, AdapterLine, AdapterRequest, AdapterResponse, AdapterResponseMessage,
    GetViewportFrameResponse,
};

pub const FRAME_MAGIC: [u8; 4] = *b"ELCX";
pub const FRAME_HEADER_LEN: usize = 21;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    Request = 1,
    Response = 2,
    Event = 3,
}

impl FrameKind {
    pub fn from_u8(value: u8) -> Result<Self, FrameError> {
        match value {
            1 => Ok(Self::Request),
            2 => Ok(Self::Response),
            3 => Ok(Self::Event),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFrame {
    pub kind: FrameKind,
    pub id: u64,
    pub json: Vec<u8>,
    pub binary: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    Io(String),
    InvalidMagic([u8; 4]),
    UnknownKind(u8),
    UnexpectedEof,
    InvalidJson(String),
    BinaryLengthMismatch {
        expected: u32,
        actual: usize,
    },
    UnexpectedFrameKind {
        expected: FrameKind,
        actual: FrameKind,
    },
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "adapter frame I/O error: {message}"),
            Self::InvalidMagic(magic) => {
                write!(formatter, "invalid adapter frame magic: {:02x?}", magic)
            }
            Self::UnknownKind(kind) => write!(formatter, "unknown adapter frame kind: {kind}"),
            Self::UnexpectedEof => write!(formatter, "unexpected end of adapter frame stream"),
            Self::InvalidJson(message) => {
                write!(formatter, "invalid adapter frame JSON: {message}")
            }
            Self::BinaryLengthMismatch { expected, actual } => write!(
                formatter,
                "adapter binary length mismatch: expected {expected}, got {actual}"
            ),
            Self::UnexpectedFrameKind { expected, actual } => write!(
                formatter,
                "unexpected adapter frame kind: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for FrameError {}

impl AdapterFrame {
    pub fn encode(&self) -> Vec<u8> {
        let json_len = u32::try_from(self.json.len()).unwrap_or(u32::MAX);
        let bin_len = u32::try_from(self.binary.len()).unwrap_or(u32::MAX);
        let mut bytes = Vec::with_capacity(FRAME_HEADER_LEN + self.json.len() + self.binary.len());
        bytes.extend_from_slice(&FRAME_MAGIC);
        bytes.push(self.kind as u8);
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&json_len.to_le_bytes());
        bytes.extend_from_slice(&bin_len.to_le_bytes());
        bytes.extend_from_slice(&self.json);
        bytes.extend_from_slice(&self.binary);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), FrameError> {
        if bytes.len() < FRAME_HEADER_LEN {
            return Err(FrameError::UnexpectedEof);
        }
        let mut magic = [0_u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != FRAME_MAGIC {
            return Err(FrameError::InvalidMagic(magic));
        }
        let kind = FrameKind::from_u8(bytes[4])?;
        let id = u64::from_le_bytes(bytes[5..13].try_into().unwrap_or([0; 8]));
        let json_len = u32::from_le_bytes(bytes[13..17].try_into().unwrap_or([0; 4])) as usize;
        let bin_len = u32::from_le_bytes(bytes[17..21].try_into().unwrap_or([0; 4])) as usize;
        let total = FRAME_HEADER_LEN
            .checked_add(json_len)
            .and_then(|value| value.checked_add(bin_len))
            .ok_or(FrameError::UnexpectedEof)?;
        if bytes.len() < total {
            return Err(FrameError::UnexpectedEof);
        }
        let json_start = FRAME_HEADER_LEN;
        let bin_start = json_start + json_len;
        Ok((
            Self {
                kind,
                id,
                json: bytes[json_start..bin_start].to_vec(),
                binary: bytes[bin_start..total].to_vec(),
            },
            total,
        ))
    }
}

pub fn write_frame<W: Write>(writer: &mut W, frame: &AdapterFrame) -> Result<(), FrameError> {
    writer
        .write_all(&frame.encode())
        .and_then(|()| writer.flush())
        .map_err(|error| FrameError::Io(error.to_string()))
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<AdapterFrame>, FrameError> {
    let mut header = [0_u8; FRAME_HEADER_LEN];
    match read_exact_or_eof(reader, &mut header)? {
        ReadOutcome::Eof => return Ok(None),
        ReadOutcome::Complete => {}
    }
    let mut magic = [0_u8; 4];
    magic.copy_from_slice(&header[0..4]);
    if magic != FRAME_MAGIC {
        return Err(FrameError::InvalidMagic(magic));
    }
    let kind = FrameKind::from_u8(header[4])?;
    let id = u64::from_le_bytes(header[5..13].try_into().unwrap_or([0; 8]));
    let json_len = u32::from_le_bytes(header[13..17].try_into().unwrap_or([0; 4])) as usize;
    let bin_len = u32::from_le_bytes(header[17..21].try_into().unwrap_or([0; 4])) as usize;
    let mut json = vec![0_u8; json_len];
    let mut binary = vec![0_u8; bin_len];
    if json_len > 0 {
        reader.read_exact(&mut json).map_err(map_read_error)?;
    }
    if bin_len > 0 {
        reader.read_exact(&mut binary).map_err(map_read_error)?;
    }
    Ok(Some(AdapterFrame {
        kind,
        id,
        json,
        binary,
    }))
}

enum ReadOutcome {
    Complete,
    Eof,
}

fn read_exact_or_eof<R: Read>(
    reader: &mut R,
    buffer: &mut [u8],
) -> Result<ReadOutcome, FrameError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) if filled == 0 => return Ok(ReadOutcome::Eof),
            Ok(0) => return Err(FrameError::UnexpectedEof),
            Ok(count) => filled += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(FrameError::Io(error.to_string())),
        }
    }
    Ok(ReadOutcome::Complete)
}

fn map_read_error(error: std::io::Error) -> FrameError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        FrameError::UnexpectedEof
    } else {
        FrameError::Io(error.to_string())
    }
}

pub fn encode_request_frame(request: &AdapterRequest) -> Result<AdapterFrame, FrameError> {
    let json =
        serde_json::to_vec(request).map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    Ok(AdapterFrame {
        kind: FrameKind::Request,
        id: request.request_id.0,
        json,
        binary: Vec::new(),
    })
}

pub fn decode_request_frame(frame: &AdapterFrame) -> Result<AdapterRequest, FrameError> {
    if frame.kind != FrameKind::Request {
        return Err(FrameError::UnexpectedFrameKind {
            expected: FrameKind::Request,
            actual: frame.kind,
        });
    }
    serde_json::from_slice(&frame.json).map_err(|error| FrameError::InvalidJson(error.to_string()))
}

pub fn encode_response_frame(response: &AdapterResponse) -> Result<AdapterFrame, FrameError> {
    let mut response = response.clone();
    let binary = take_viewport_pixels(&mut response.message);
    let json = serde_json::to_vec(&AdapterLine::Response(response.clone()))
        .map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    Ok(AdapterFrame {
        kind: FrameKind::Response,
        id: response.request_id.0,
        json,
        binary,
    })
}

pub fn encode_event_frame(event: &AdapterEvent) -> Result<AdapterFrame, FrameError> {
    let json = serde_json::to_vec(&AdapterLine::Event(event.clone()))
        .map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    Ok(AdapterFrame {
        kind: FrameKind::Event,
        id: 0,
        json,
        binary: Vec::new(),
    })
}

pub fn decode_adapter_frame(frame: &AdapterFrame) -> Result<AdapterLine, FrameError> {
    match frame.kind {
        FrameKind::Response | FrameKind::Event => {}
        FrameKind::Request => {
            return Err(FrameError::UnexpectedFrameKind {
                expected: FrameKind::Response,
                actual: frame.kind,
            });
        }
    }
    let mut line: AdapterLine = serde_json::from_slice(&frame.json)
        .map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    if let AdapterLine::Response(AdapterResponse {
        message: AdapterResponseMessage::GetViewportFrame(response),
        ..
    }) = &mut line
    {
        attach_viewport_pixels(response, &frame.binary)?;
    }
    Ok(line)
}

fn take_viewport_pixels(message: &mut AdapterResponseMessage) -> Vec<u8> {
    let AdapterResponseMessage::GetViewportFrame(response) = message else {
        return Vec::new();
    };
    let pixels = std::mem::take(&mut response.pixels);
    response.byte_len = u32::try_from(pixels.len()).unwrap_or(u32::MAX);
    pixels
}

fn attach_viewport_pixels(
    response: &mut GetViewportFrameResponse,
    binary: &[u8],
) -> Result<(), FrameError> {
    if response.byte_len as usize != binary.len() {
        return Err(FrameError::BinaryLengthMismatch {
            expected: response.byte_len,
            actual: binary.len(),
        });
    }
    response.pixels = binary.to_vec();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterRequest, AdapterRequestId, AdapterRequestMessage, AdapterResponse,
        AdapterResponseMessage, AdapterViewportId, GetDiagnosticsRequest, GetViewportFrameResponse,
        ViewportFrameResponseStatus,
    };
    use elcarax_core::ViewportFrameFormat;
    use std::io::Cursor;

    #[test]
    fn request_frame_round_trips() {
        let request = AdapterRequest::new(
            AdapterRequestId::new(7),
            AdapterRequestMessage::GetDiagnostics(GetDiagnosticsRequest),
        );
        let frame = match encode_request_frame(&request) {
            Ok(frame) => frame,
            Err(error) => panic!("encode should succeed: {error}"),
        };
        let encoded = frame.encode();
        let (decoded_frame, len) = match AdapterFrame::decode(&encoded) {
            Ok(value) => value,
            Err(error) => panic!("decode bytes should succeed: {error}"),
        };
        assert_eq!(len, encoded.len());
        let decoded = match decode_request_frame(&decoded_frame) {
            Ok(value) => value,
            Err(error) => panic!("decode request should succeed: {error}"),
        };
        assert_eq!(decoded, request);
    }

    #[test]
    fn viewport_response_keeps_pixels_in_binary_segment() {
        let pixels = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let response = AdapterResponse::new(
            AdapterRequestId::new(3),
            AdapterResponseMessage::GetViewportFrame(GetViewportFrameResponse {
                viewport_id: AdapterViewportId(1),
                width: 2,
                height: 1,
                format: ViewportFrameFormat::Rgba8Unorm,
                byte_len: 0,
                pixels: pixels.clone(),
                diagnostics: Vec::new(),
                status: ViewportFrameResponseStatus::Available,
            }),
        );
        let frame = match encode_response_frame(&response) {
            Ok(frame) => frame,
            Err(error) => panic!("encode should succeed: {error}"),
        };
        assert_eq!(frame.binary, pixels);
        assert!(!String::from_utf8_lossy(&frame.json).contains("\"pixels\""));
        let line = match decode_adapter_frame(&frame) {
            Ok(value) => value,
            Err(error) => panic!("decode should succeed: {error}"),
        };
        let AdapterLine::Response(AdapterResponse {
            message: AdapterResponseMessage::GetViewportFrame(decoded),
            ..
        }) = line
        else {
            panic!("expected viewport response");
        };
        assert_eq!(decoded.pixels, pixels);
        assert_eq!(decoded.byte_len, 8);
    }

    #[test]
    fn write_and_read_frame_stream() {
        let request = AdapterRequest::new(
            AdapterRequestId::new(1),
            AdapterRequestMessage::GetDiagnostics(GetDiagnosticsRequest),
        );
        let frame = match encode_request_frame(&request) {
            Ok(frame) => frame,
            Err(error) => panic!("encode should succeed: {error}"),
        };
        let mut buffer = Vec::new();
        if let Err(error) = write_frame(&mut buffer, &frame) {
            panic!("write should succeed: {error}");
        }
        let mut cursor = Cursor::new(buffer);
        let decoded = match read_frame(&mut cursor) {
            Ok(Some(frame)) => frame,
            Ok(None) => panic!("frame should be present"),
            Err(error) => panic!("read should succeed: {error}"),
        };
        let decoded_request = match decode_request_frame(&decoded) {
            Ok(value) => value,
            Err(error) => panic!("decode should succeed: {error}"),
        };
        assert_eq!(decoded_request, request);
        match read_frame(&mut cursor) {
            Ok(None) => {}
            Ok(Some(_)) => panic!("expected eof"),
            Err(error) => panic!("eof read should succeed: {error}"),
        }
    }
}
