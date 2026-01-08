use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::{self, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard error: {0}")]
    Arboard(#[from] arboard::Error),
    #[error("Invalid hex string: {0}")]
    InvalidHex(String),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// HEXコピーのフォーマット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HexFormat {
    /// スペース区切り: "48 65 6C 6C 6F"
    #[default]
    Spaced,
    /// 連続: "48656C6C6F"
    Continuous,
    /// C言語配列: "0x48, 0x65, 0x6C, 0x6C, 0x6F"
    CArray,
}

/// バイト列をHEX文字列に変換
pub fn bytes_to_hex(bytes: &[u8], format: HexFormat) -> String {
    match format {
        HexFormat::Spaced => bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" "),
        HexFormat::Continuous => bytes.iter().map(|b| format!("{:02X}", b)).collect(),
        HexFormat::CArray => {
            let inner = bytes
                .iter()
                .map(|b| format!("0x{:02X}", b))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", inner)
        }
    }
}

/// HEX文字列をバイト列に変換
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ClipboardError> {
    // スペース、カンマ、0x プレフィックスを除去
    let cleaned: String = hex
        .replace(" ", "")
        .replace(",", "")
        .replace("0x", "")
        .replace("0X", "")
        .replace("{", "")
        .replace("}", "")
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();

    if cleaned.len() % 2 != 0 {
        return Err(ClipboardError::InvalidHex(
            "Hex string must have even length".to_string(),
        ));
    }

    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| ClipboardError::InvalidHex(cleaned[i..i + 2].to_string()))
        })
        .collect()
}

/// クリップボードにHEX文字列をコピー
pub fn copy_hex(bytes: &[u8], format: HexFormat) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    let hex = bytes_to_hex(bytes, format);
    clipboard.set_text(hex)?;
    Ok(())
}

/// クリップボードからHEX文字列をペースト
pub fn paste_hex() -> Result<Vec<u8>, ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    let text = clipboard.get_text()?;
    hex_to_bytes(&text)
}

/// クリップボードにテキストをコピー
pub fn copy_text(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

// =============================================================================
// OSC 52 クリップボード連携
// =============================================================================

/// tmux環境かどうかを検出
fn is_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// screen環境かどうかを検出
fn is_screen() -> bool {
    std::env::var("STY").is_ok()
}

/// OSC 52シーケンスを生成
/// OSC 52 ; Pc ; Pd ST
/// Pc = "c" (clipboard)
/// Pd = base64エンコードされたデータ
fn build_osc52_sequence(data: &[u8]) -> Vec<u8> {
    let encoded = STANDARD.encode(data);
    let mut seq = Vec::new();

    // OSC 52 ; c ; <base64> ST
    // OSC = ESC ]
    // ST = ESC \ (または BEL = 0x07)
    seq.extend_from_slice(b"\x1b]52;c;");
    seq.extend_from_slice(encoded.as_bytes());
    seq.push(0x07); // BEL as ST (より互換性が高い)

    seq
}

/// tmuxパススルーでOSC 52をラップ
/// DCS tmux ; <escaped-osc52> ST
fn wrap_for_tmux(osc52: &[u8]) -> Vec<u8> {
    let mut seq = Vec::new();

    // DCS = ESC P
    // tmux; の後にESCをダブルにしたOSC52シーケンス
    // ST = ESC \
    seq.extend_from_slice(b"\x1bPtmux;");

    // OSC52内のESCをダブルにする
    for &byte in osc52 {
        if byte == 0x1b {
            seq.push(0x1b);
            seq.push(0x1b);
        } else {
            seq.push(byte);
        }
    }

    seq.extend_from_slice(b"\x1b\\");

    seq
}

/// screenパススルーでOSC 52をラップ
fn wrap_for_screen(osc52: &[u8]) -> Vec<u8> {
    let mut seq = Vec::new();

    // DCS = ESC P
    seq.extend_from_slice(b"\x1bP");
    seq.extend_from_slice(osc52);
    seq.extend_from_slice(b"\x1b\\");

    seq
}

/// OSC 52を使ってターミナルクリップボードにコピー
pub fn copy_to_terminal(data: &[u8]) -> Result<(), ClipboardError> {
    let osc52 = build_osc52_sequence(data);

    let sequence = if is_tmux() {
        wrap_for_tmux(&osc52)
    } else if is_screen() {
        wrap_for_screen(&osc52)
    } else {
        osc52
    };

    let mut stdout = io::stdout().lock();
    stdout.write_all(&sequence)?;
    stdout.flush()?;

    Ok(())
}

/// OSC 52を使ってテキストをターミナルクリップボードにコピー
pub fn copy_text_to_terminal(text: &str) -> Result<(), ClipboardError> {
    copy_to_terminal(text.as_bytes())
}

/// HEXフォーマットでターミナルクリップボードにコピー
pub fn copy_hex_to_terminal(bytes: &[u8], format: HexFormat) -> Result<(), ClipboardError> {
    let hex = bytes_to_hex(bytes, format);
    copy_to_terminal(hex.as_bytes())
}

/// 両方のクリップボード（システム + ターミナル）にコピー
pub fn copy_hex_to_all(bytes: &[u8], format: HexFormat) -> Result<(), ClipboardError> {
    // まずシステムクリップボードにコピー
    let result = copy_hex(bytes, format);

    // ターミナルクリップボードにもコピー（失敗しても無視）
    let _ = copy_hex_to_terminal(bytes, format);

    result
}

/// 両方のクリップボード（システム + ターミナル）にテキストをコピー
pub fn copy_text_to_all(text: &str) -> Result<(), ClipboardError> {
    // まずシステムクリップボードにコピー
    let result = copy_text(text);

    // ターミナルクリップボードにもコピー（失敗しても無視）
    let _ = copy_text_to_terminal(text);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_hex() {
        let bytes = b"Hello";
        assert_eq!(bytes_to_hex(bytes, HexFormat::Spaced), "48 65 6C 6C 6F");
        assert_eq!(bytes_to_hex(bytes, HexFormat::Continuous), "48656C6C6F");
        assert_eq!(
            bytes_to_hex(bytes, HexFormat::CArray),
            "{ 0x48, 0x65, 0x6C, 0x6C, 0x6F }"
        );
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("48 65 6C 6C 6F").unwrap(), b"Hello");
        assert_eq!(hex_to_bytes("48656C6C6F").unwrap(), b"Hello");
        assert_eq!(
            hex_to_bytes("0x48, 0x65, 0x6C, 0x6C, 0x6F").unwrap(),
            b"Hello"
        );
    }
}
