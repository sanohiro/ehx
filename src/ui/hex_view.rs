use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use super::Colors;
use crate::encoding::{decode_for_display, CharEncoding};

/// 表示モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Hex,
    Ascii,
}

/// HEX/ASCII表示ウィジェット
pub struct HexView<'a> {
    /// 表示するデータ
    data: &'a [u8],
    /// 表示開始オフセット
    offset: usize,
    /// 1行あたりのバイト数
    bytes_per_row: usize,
    /// カーソル位置
    cursor: usize,
    /// 選択範囲（開始, 終了）
    selection: Option<(usize, usize)>,
    /// 現在の表示モード
    mode: ViewMode,
    /// 文字エンコーディング
    encoding: CharEncoding,
    /// アドレス表示の基数（16進数 or 10進数）
    addr_radix: u8,
}

impl<'a> HexView<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            bytes_per_row: 16,
            cursor: 0,
            selection: None,
            mode: ViewMode::Hex,
            encoding: CharEncoding::Utf8,
            addr_radix: 16,
        }
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn bytes_per_row(mut self, bytes: usize) -> Self {
        self.bytes_per_row = bytes;
        self
    }

    pub fn cursor(mut self, cursor: usize) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn selection(mut self, selection: Option<(usize, usize)>) -> Self {
        self.selection = selection;
        self
    }

    pub fn mode(mut self, mode: ViewMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn encoding(mut self, encoding: CharEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// アドレス文字列を生成
    fn format_addr(&self, addr: usize) -> String {
        if self.addr_radix == 16 {
            format!("{:08X}", addr)
        } else {
            format!("{:010}", addr)
        }
    }

    /// バイト値に応じた色を取得
    fn byte_color(&self, byte: u8) -> Color {
        match byte {
            0x00 => Colors::HEX_ZERO,
            0xFF => Colors::HEX_HIGH,
            0x20..=0x7E => Colors::HEX_PRINTABLE,
            _ => Colors::HEX_NORMAL,
        }
    }

    /// 前の行からはみ出した文字の継続バイト数を計算
    fn count_continuation_bytes(&self, row_start: usize) -> usize {
        if row_start == 0 || self.data.is_empty() {
            return 0;
        }

        // 前の数バイトを調べて、行境界をまたぐ文字があるかチェック
        let lookahead = 4;
        let check_start = row_start.saturating_sub(lookahead);
        let end = row_start.min(self.data.len());
        if check_start >= end {
            return 0;
        }
        let check_bytes = &self.data[check_start..end];

        if check_bytes.is_empty() {
            return 0;
        }

        // デコードして最後の文字が行をまたぐかチェック
        let decoded = decode_for_display(check_bytes, self.encoding);

        let mut pos = 0;
        let mut last_char_end = 0;
        while pos < decoded.len() {
            if let Some(ref dc) = decoded[pos] {
                last_char_end = check_start + pos + dc.byte_len;
                pos += dc.byte_len;
            } else {
                pos += 1;
            }
        }

        // 最後の文字が row_start を超えていれば、その分が継続バイト
        if last_char_end > row_start {
            last_char_end - row_start
        } else {
            0
        }
    }

    /// 1行分のデータを描画
    fn render_row(&self, row_offset: usize, area: Rect, buf: &mut Buffer) {
        let row_start = self.offset + row_offset * self.bytes_per_row;
        let row_end = (row_start + self.bytes_per_row).min(self.data.len());

        // 前の行からはみ出した文字の継続バイト数
        let skip_bytes = self.count_continuation_bytes(row_start);

        // EOF行も描画可能にする（カーソルがEOF位置にある場合）
        let eof_pos = self.data.len();
        let cursor_at_eof = self.cursor == eof_pos;

        if row_start > self.data.len() {
            return;
        }

        // データがなく、かつカーソルもこの行にない場合はスキップ
        if row_start >= self.data.len() && !cursor_at_eof {
            return;
        }

        let mut x = area.x;
        let y = area.y;

        // アドレス表示
        let addr_str = self.format_addr(row_start);
        buf.set_string(x, y, &addr_str, Style::default().fg(Colors::ADDR));
        x += addr_str.len() as u16 + 2;

        // HEX表示
        for i in row_start..row_start + self.bytes_per_row {
            if i < row_end {
                let byte = self.data[i];
                let hex = format!("{:02X}", byte);

                let mut style = Style::default().fg(self.byte_color(byte));

                // カーソル位置のハイライト
                if i == self.cursor && self.mode == ViewMode::Hex {
                    style = style.bg(Colors::CURSOR_BG).fg(Colors::CURSOR);
                }
                // 選択範囲のハイライト
                else if let Some((start, end)) = self.selection {
                    if i >= start && i <= end {
                        style = style.bg(Colors::SELECTION_BG);
                    }
                }

                buf.set_string(x, y, &hex, style);
            } else if i == eof_pos && i == self.cursor && self.mode == ViewMode::Hex {
                // EOF位置のカーソル（HEXモード）
                buf.set_string(x, y, "__", Style::default().bg(Colors::CURSOR_BG).fg(Colors::CURSOR));
            } else {
                buf.set_string(x, y, "  ", Style::default());
            }
            x += 3; // "XX "
        }

        x += 1; // 区切りスペース

        // ASCII表示（エンコーディングに従ってデコード）
        // 行末のマルチバイト文字を正しく表示するため、次の行のバイトも含めてデコード
        let lookahead = 4; // UTF-8/UTF-16の最大バイト数
        let decode_end = (row_end + lookahead).min(self.data.len());
        let row_bytes = if decode_end > row_start {
            &self.data[row_start..decode_end]
        } else {
            &[]
        };
        let decoded = decode_for_display(row_bytes, self.encoding);

        let mut byte_idx = 0;
        // 前の行からはみ出した文字の継続バイトをスキップ
        while byte_idx < skip_bytes && byte_idx < self.bytes_per_row {
            x += 1;
            byte_idx += 1;
        }

        while byte_idx < self.bytes_per_row {
            let abs_idx = row_start + byte_idx;

            if byte_idx < decoded.len() {
                if let Some(ref dc) = decoded[byte_idx] {
                    // この位置に文字がある
                    let mut style = Style::default().fg(Colors::ASCII_NORMAL);

                    // カーソル位置のハイライト
                    let cursor_in_char = self.cursor >= abs_idx
                        && self.cursor < abs_idx + dc.byte_len;
                    if cursor_in_char && self.mode == ViewMode::Ascii {
                        style = style.bg(Colors::CURSOR_BG).fg(Colors::CURSOR);
                    }
                    // 選択範囲のハイライト
                    else if let Some((start, end)) = self.selection {
                        if abs_idx >= start && abs_idx <= end {
                            style = style.bg(Colors::SELECTION_BG);
                        }
                    }

                    // 文字を表示
                    buf.set_string(x, y, &dc.display, style);

                    // この行内のバイト数を計算
                    let bytes_in_row = dc.byte_len.min(self.bytes_per_row - byte_idx);

                    // 表示幅分進める
                    // 行をはみ出す文字は表示幅だけ進める（はみ出し表示）
                    let advance = if dc.byte_len <= bytes_in_row {
                        // 行内に収まる場合
                        dc.width.min(dc.byte_len)
                    } else {
                        // 行をはみ出す場合：文字の表示幅を使用
                        dc.width
                    };
                    x += advance as u16;

                    // 残りのバイト分（行内）はスペースで埋める
                    if bytes_in_row > advance {
                        for _ in advance..bytes_in_row {
                            x += 1;
                        }
                    }

                    byte_idx += bytes_in_row;
                } else {
                    // None = 継続バイト（前の文字の一部）- スキップ済みのはず
                    x += 1;
                    byte_idx += 1;
                }
            } else if abs_idx == eof_pos && abs_idx == self.cursor && self.mode == ViewMode::Ascii {
                // EOF位置のカーソル（ASCIIモード）
                buf.set_string(x, y, "_", Style::default().bg(Colors::CURSOR_BG).fg(Colors::CURSOR));
                x += 1;
                byte_idx += 1;
            } else {
                // データがない部分
                x += 1;
                byte_idx += 1;
            }
        }
    }
}

impl Widget for HexView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ヘッダー行を描画
        let header = format!(
            "{:8}  {:}  {:}",
            "Offset",
            (0..self.bytes_per_row)
                .map(|i| format!("{:02X}", i))
                .collect::<Vec<_>>()
                .join(" "),
            "ASCII"
        );
        buf.set_string(
            area.x,
            area.y,
            &header,
            Style::default()
                .fg(Colors::HEADER)
                .add_modifier(Modifier::BOLD),
        );

        // データ行を描画
        let visible_rows = (area.height as usize).saturating_sub(1); // ヘッダー分を引く
        for row in 0..visible_rows {
            let row_area = Rect {
                x: area.x,
                y: area.y + 1 + row as u16,
                width: area.width,
                height: 1,
            };
            self.render_row(row, row_area, buf);
        }
    }
}
