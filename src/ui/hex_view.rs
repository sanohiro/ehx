use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use super::Colors;
use crate::encoding::{byte_to_char, CharEncoding};

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

    /// 1行分のデータを描画
    fn render_row(&self, row_offset: usize, area: Rect, buf: &mut Buffer) {
        let row_start = self.offset + row_offset * self.bytes_per_row;
        let row_end = (row_start + self.bytes_per_row).min(self.data.len());

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

        // ASCII表示
        for i in row_start..row_start + self.bytes_per_row {
            if i < row_end {
                let byte = self.data[i];
                let ch = byte_to_char(byte);

                let mut style = if byte.is_ascii_graphic() || byte == b' ' {
                    Style::default().fg(Colors::ASCII_NORMAL)
                } else {
                    Style::default().fg(Colors::ASCII_CONTROL)
                };

                // カーソル位置のハイライト
                if i == self.cursor && self.mode == ViewMode::Ascii {
                    style = style.bg(Colors::CURSOR_BG).fg(Colors::CURSOR);
                }
                // 選択範囲のハイライト
                else if let Some((start, end)) = self.selection {
                    if i >= start && i <= end {
                        style = style.bg(Colors::SELECTION_BG);
                    }
                }

                buf.set_string(x, y, &ch.to_string(), style);
            } else if i == eof_pos && i == self.cursor && self.mode == ViewMode::Ascii {
                // EOF位置のカーソル（ASCIIモード）
                buf.set_string(x, y, "_", Style::default().bg(Colors::CURSOR_BG).fg(Colors::CURSOR));
            }
            x += 1;
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
