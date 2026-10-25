use serde::{Deserialize, Serialize};
use std::path::Path;

pub const BINARY_SNIFF_BYTES: usize = 8 * 1024;
pub const DEFAULT_LARGE_FILE_LIMIT_BYTES: u64 = 5 * 1024 * 1024;
pub const DEFAULT_LONG_LINE_LIMIT_CHARS: usize = 10_000;
const UTF8_BOM: &[u8; 3] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    Text,
    Image,
    Svg,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncodingKind {
    Utf8,
    Utf8Bom,
    NonUtf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInspection {
    pub text: String,
    pub encoding: EncodingKind,
    pub line_ending: LineEnding,
    pub has_mixed_line_endings: bool,
    pub has_long_lines: bool,
}

pub fn classify_file_kind(path: &Path, header_bytes: &[u8]) -> FileKind {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_ascii_lowercase();
        match lower.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => return FileKind::Image,
            "svg" => {
                if is_binary_header(header_bytes) {
                    return FileKind::Binary;
                }
                return FileKind::Svg;
            }
            _ => {}
        }
    }

    if is_binary_header(header_bytes) {
        FileKind::Binary
    } else {
        FileKind::Text
    }
}

pub fn is_binary_header(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(BINARY_SNIFF_BYTES);
    bytes[..check_len].contains(&0x00)
}

pub fn inspect_text_bytes(bytes: &[u8], long_line_limit: usize) -> TextInspection {
    let (body, has_bom) = if bytes.starts_with(UTF8_BOM) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    };

    let (line_ending, has_mixed_line_endings) = detect_line_endings(body);

    let (raw_text, encoding) = match std::str::from_utf8(body) {
        Ok(valid) => {
            let enc = if has_bom {
                EncodingKind::Utf8Bom
            } else {
                EncodingKind::Utf8
            };
            (valid.to_string(), enc)
        }
        Err(_) => (
            String::from_utf8_lossy(body).into_owned(),
            EncodingKind::NonUtf8,
        ),
    };

    let normalized_text = if raw_text.contains("\r\n") {
        raw_text.replace("\r\n", "\n")
    } else {
        raw_text
    };

    let has_long_lines = normalized_text
        .lines()
        .any(|line| line.chars().count() > long_line_limit);

    TextInspection {
        text: normalized_text,
        encoding,
        line_ending,
        has_mixed_line_endings,
        has_long_lines,
    }
}

pub fn detect_line_endings(bytes: &[u8]) -> (LineEnding, bool) {
    let mut crlf_count = 0usize;
    let mut bare_lf_count = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        if bytes[idx] == b'\r' && idx + 1 < bytes.len() && bytes[idx + 1] == b'\n' {
            crlf_count += 1;
            idx += 2;
        } else {
            if bytes[idx] == b'\n' {
                bare_lf_count += 1;
            }
            idx += 1;
        }
    }

    let mixed = crlf_count > 0 && bare_lf_count > 0;
    let dominant = if crlf_count > bare_lf_count {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    };

    (dominant, mixed)
}

pub fn encode_text_for_save(
    content: &str,
    encoding: EncodingKind,
    line_ending: LineEnding,
) -> Vec<u8> {
    let normalized_lf = if content.contains("\r\n") {
        content.replace("\r\n", "\n")
    } else {
        content.to_string()
    };

    let body = match line_ending {
        LineEnding::Lf => normalized_lf,
        LineEnding::Crlf => normalized_lf.replace('\n', "\r\n"),
    };

    let mut out = Vec::with_capacity(body.len() + 3);
    if encoding == EncodingKind::Utf8Bom {
        out.extend_from_slice(UTF8_BOM);
    }
    out.extend_from_slice(body.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_binary_from_null_byte_in_first_8kb() {
        let mut text_data = vec![b'a'; 10_000];
        assert!(!is_binary_header(&text_data));

        text_data[8192] = 0x00;
        assert!(!is_binary_header(&text_data));

        text_data[512] = 0x00;
        assert!(is_binary_header(&text_data));
    }

    #[test]
    fn classifies_image_svg_text_and_binary() {
        assert_eq!(
            classify_file_kind(Path::new("logo.png"), &[0x89, b'P', b'N', b'G', 0x00]),
            FileKind::Image
        );
        assert_eq!(
            classify_file_kind(Path::new("icon.svg"), b"<svg></svg>"),
            FileKind::Svg
        );
        assert_eq!(
            classify_file_kind(Path::new("archive.bin"), &[0x01, 0x00, 0x02]),
            FileKind::Binary
        );
        assert_eq!(
            classify_file_kind(Path::new("main.rs"), b"fn main() {}\n"),
            FileKind::Text
        );
    }

    #[test]
    fn inspects_and_preserves_bom_and_crlf_on_save() {
        let mut raw = Vec::from(&UTF8_BOM[..]);
        raw.extend_from_slice(b"first\r\nsecond\r\n");

        let info = inspect_text_bytes(&raw, 100);
        assert_eq!(info.encoding, EncodingKind::Utf8Bom);
        assert_eq!(info.line_ending, LineEnding::Crlf);
        assert!(!info.has_mixed_line_endings);
        assert_eq!(info.text, "first\nsecond\n");

        let reencoded = encode_text_for_save(&info.text, info.encoding, info.line_ending);
        assert_eq!(reencoded, raw);
    }

    #[test]
    fn detects_mixed_line_endings_non_utf8_and_long_lines() {
        let mixed = b"line1\r\nline2\nline3\r\n";
        let info = inspect_text_bytes(mixed, 4);
        assert_eq!(info.line_ending, LineEnding::Crlf);
        assert!(info.has_mixed_line_endings);
        assert!(info.has_long_lines);

        let non_utf8 = &[0xFF, 0xFE, b'a', b'\n'];
        let bad_info = inspect_text_bytes(non_utf8, 100);
        assert_eq!(bad_info.encoding, EncodingKind::NonUtf8);
    }
}
