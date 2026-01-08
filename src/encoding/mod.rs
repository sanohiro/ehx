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
