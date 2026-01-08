#![allow(dead_code)]

use encoding_rs::Encoding;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// サポートする文字エンコーディング
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharEncoding {
    #[default]
    Utf8,
    Utf16Le,
    Utf16Be,
    ShiftJis,
    EucJp,
    Iso2022Jp,
    Ascii,
    Latin1,
}

impl CharEncoding {
    /// エンコーディング名を取得
    pub fn name(&self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::ShiftJis => "Shift-JIS",
            Self::EucJp => "EUC-JP",
            Self::Iso2022Jp => "ISO-2022-JP",
            Self::Ascii => "ASCII",
            Self::Latin1 => "Latin-1",
        }
    }

    /// encoding_rsのEncodingを取得
    pub fn to_encoding(&self) -> &'static Encoding {
        match self {
            Self::Utf8 => encoding_rs::UTF_8,
            Self::Utf16Le => encoding_rs::UTF_16LE,
            Self::Utf16Be => encoding_rs::UTF_16BE,
            Self::ShiftJis => encoding_rs::SHIFT_JIS,
            Self::EucJp => encoding_rs::EUC_JP,
            Self::Iso2022Jp => encoding_rs::ISO_2022_JP,
            Self::Ascii | Self::Latin1 => encoding_rs::WINDOWS_1252,
        }
    }

    /// 次のエンコーディングに切り替え
    pub fn next(&self) -> Self {
        match self {
            Self::Utf8 => Self::Utf16Le,
            Self::Utf16Le => Self::Utf16Be,
            Self::Utf16Be => Self::ShiftJis,
            Self::ShiftJis => Self::EucJp,
            Self::EucJp => Self::Iso2022Jp,
            Self::Iso2022Jp => Self::Ascii,
            Self::Ascii => Self::Latin1,
            Self::Latin1 => Self::Utf8,
        }
    }
}

/// バイト列を文字列にデコード
pub fn decode_bytes(bytes: &[u8], encoding: CharEncoding) -> String {
    let enc = encoding.to_encoding();
    let (result, _, _) = enc.decode(bytes);
    result.into_owned()
}

/// 文字列をバイト列にエンコード
/// エンコードできない文字は置換文字になる
pub fn encode_string(s: &str, encoding: CharEncoding) -> Vec<u8> {
    let enc = encoding.to_encoding();
    let (result, _, _) = enc.encode(s);
    result.into_owned()
}

/// 文字をバイト列にエンコード
/// エンコードできない場合は None を返す
pub fn encode_char(ch: char, encoding: CharEncoding) -> Option<Vec<u8>> {
    let s: String = ch.to_string();
    let enc = encoding.to_encoding();
    let (result, _, had_errors) = enc.encode(&s);
    if had_errors {
        // エンコードエラー（置換文字が使われた）
        None
    } else {
        Some(result.into_owned())
    }
}

/// 1バイトを表示用文字に変換（ASCII範囲外は'.'）
pub fn byte_to_char(byte: u8) -> char {
    if byte.is_ascii_graphic() || byte == b' ' {
        byte as char
    } else {
        '.'
    }
}

/// 書記素クラスタの表示幅を計算
pub fn grapheme_width(s: &str) -> usize {
    s.graphemes(true)
        .map(|g| UnicodeWidthStr::width(g))
        .sum()
}

/// 文字列の書記素クラスタを取得
pub fn graphemes(s: &str) -> Vec<&str> {
    s.graphemes(true).collect()
}

/// デコード結果の1文字分の情報
#[derive(Debug, Clone)]
pub struct DecodedChar {
    /// 表示する文字列（書記素クラスタ）
    pub display: String,
    /// この文字が占めるバイト数
    pub byte_len: usize,
    /// 表示幅（半角=1, 全角=2）
    pub width: usize,
}

/// バイト列をデコードして文字ごとの情報を返す
/// 各バイト位置に対応する表示情報を返す
pub fn decode_for_display(bytes: &[u8], encoding: CharEncoding) -> Vec<Option<DecodedChar>> {
    if bytes.is_empty() {
        return vec![];
    }

    let mut result = vec![None; bytes.len()];

    match encoding {
        CharEncoding::Utf8 => decode_utf8_for_display(bytes, &mut result),
        CharEncoding::ShiftJis | CharEncoding::EucJp => {
            decode_with_encoding_rs(bytes, encoding, &mut result)
        }
        CharEncoding::Utf16Le | CharEncoding::Utf16Be => {
            decode_utf16_for_display(bytes, encoding, &mut result)
        }
        _ => {
            // ASCII, Latin1: 1バイト1文字
            for (i, &byte) in bytes.iter().enumerate() {
                let ch = if byte.is_ascii_graphic() || byte == b' ' {
                    (byte as char).to_string()
                } else if byte < 0x20 || byte == 0x7F {
                    ".".to_string()
                } else {
                    // Latin1 extended
                    char::from_u32(byte as u32)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| ".".to_string())
                };
                result[i] = Some(DecodedChar {
                    display: ch,
                    byte_len: 1,
                    width: 1,
                });
            }
        }
    }

    result
}

/// UTF-8デコード
fn decode_utf8_for_display(bytes: &[u8], result: &mut [Option<DecodedChar>]) {
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        let char_len = utf8_char_len(byte);

        if char_len == 0 || i + char_len > bytes.len() {
            // 不正なバイト
            result[i] = Some(DecodedChar {
                display: ".".to_string(),
                byte_len: 1,
                width: 1,
            });
            i += 1;
            continue;
        }

        let slice = &bytes[i..i + char_len];
        match std::str::from_utf8(slice) {
            Ok(s) => {
                let graphemes: Vec<&str> = s.graphemes(true).collect();
                if let Some(g) = graphemes.first() {
                    let width = UnicodeWidthStr::width(*g).max(1);
                    let display = if is_displayable(g) {
                        g.to_string()
                    } else {
                        ".".to_string()
                    };
                    result[i] = Some(DecodedChar {
                        display,
                        byte_len: char_len,
                        width,
                    });
                }
            }
            Err(_) => {
                result[i] = Some(DecodedChar {
                    display: ".".to_string(),
                    byte_len: 1,
                    width: 1,
                });
                i += 1;
                continue;
            }
        }
        i += char_len;
    }
}

/// UTF-8の文字バイト長を取得
fn utf8_char_len(first_byte: u8) -> usize {
    match first_byte {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 0, // 継続バイトまたは不正
    }
}

/// UTF-16デコード
fn decode_utf16_for_display(bytes: &[u8], encoding: CharEncoding, result: &mut [Option<DecodedChar>]) {
    let is_le = encoding == CharEncoding::Utf16Le;
    let mut i = 0;

    while i + 1 < bytes.len() {
        let code_unit = if is_le {
            u16::from_le_bytes([bytes[i], bytes[i + 1]])
        } else {
            u16::from_be_bytes([bytes[i], bytes[i + 1]])
        };

        // サロゲートペアチェック
        if (0xD800..=0xDBFF).contains(&code_unit) && i + 3 < bytes.len() {
            let low = if is_le {
                u16::from_le_bytes([bytes[i + 2], bytes[i + 3]])
            } else {
                u16::from_be_bytes([bytes[i + 2], bytes[i + 3]])
            };

            if (0xDC00..=0xDFFF).contains(&low) {
                // サロゲートペア
                let code_point = 0x10000
                    + (((code_unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
                if let Some(ch) = char::from_u32(code_point) {
                    let s = ch.to_string();
                    let width = UnicodeWidthStr::width(s.as_str()).max(1);
                    result[i] = Some(DecodedChar {
                        display: s,
                        byte_len: 4,
                        width,
                    });
                    i += 4;
                    continue;
                }
            }
        }

        // 通常の2バイト文字
        if let Some(ch) = char::from_u32(code_unit as u32) {
            let s = ch.to_string();
            let width = if is_displayable(&s) {
                UnicodeWidthStr::width(s.as_str()).max(1)
            } else {
                1
            };
            let display = if is_displayable(&s) { s } else { ".".to_string() };
            result[i] = Some(DecodedChar {
                display,
                byte_len: 2,
                width,
            });
        } else {
            result[i] = Some(DecodedChar {
                display: ".".to_string(),
                byte_len: 2,
                width: 1,
            });
        }
        i += 2;
    }

    // 端数バイト
    if i < bytes.len() {
        result[i] = Some(DecodedChar {
            display: ".".to_string(),
            byte_len: 1,
            width: 1,
        });
    }
}

/// encoding_rsを使ったデコード（Shift-JIS, EUC-JP等）
fn decode_with_encoding_rs(bytes: &[u8], encoding: CharEncoding, result: &mut [Option<DecodedChar>]) {
    let enc = encoding.to_encoding();

    let mut i = 0;
    while i < bytes.len() {
        // 1〜4バイトを試してデコード
        let mut decoded = false;
        for len in 1..=4.min(bytes.len() - i) {
            let slice = &bytes[i..i + len];
            let (cow, _, had_errors) = enc.decode(slice);

            if !had_errors && !cow.is_empty() {
                let s = cow.into_owned();
                let graphemes: Vec<&str> = s.graphemes(true).collect();

                if let Some(g) = graphemes.first() {
                    // 完全な文字がデコードできたか確認
                    let (encoded, _, _) = enc.encode(g);
                    if encoded.len() == len {
                        let width = UnicodeWidthStr::width(*g).max(1);
                        let display = if is_displayable(g) {
                            g.to_string()
                        } else {
                            ".".to_string()
                        };
                        result[i] = Some(DecodedChar {
                            display,
                            byte_len: len,
                            width,
                        });
                        i += len;
                        decoded = true;
                        break;
                    }
                }
            }
        }

        if !decoded {
            result[i] = Some(DecodedChar {
                display: ".".to_string(),
                byte_len: 1,
                width: 1,
            });
            i += 1;
        }
    }
}

/// 表示可能な文字かどうか
fn is_displayable(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let ch = s.chars().next().unwrap();
    // 制御文字は表示しない
    if ch.is_control() && ch != ' ' {
        return false;
    }
    // 代替文字は表示しない
    if ch == '\u{FFFD}' {
        return false;
    }
    true
}
