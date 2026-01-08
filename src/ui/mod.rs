#![allow(dead_code)]

mod hex_view;

pub use hex_view::{HexView, ViewMode};

use ratatui::style::Color;

/// デフォルトカラー設定
pub struct Colors;

impl Colors {
    pub const ADDR: Color = Color::Cyan;
    pub const HEX_NORMAL: Color = Color::White;
    pub const HEX_ZERO: Color = Color::DarkGray;
    pub const HEX_HIGH: Color = Color::Red;
    pub const HEX_PRINTABLE: Color = Color::Green;
    pub const ASCII_NORMAL: Color = Color::White;
    pub const ASCII_CONTROL: Color = Color::DarkGray;
    pub const CURSOR: Color = Color::Black;
    pub const CURSOR_BG: Color = Color::Yellow;
    pub const SELECTION_BG: Color = Color::Blue;
    pub const MODIFIED: Color = Color::Magenta;
    pub const HEADER: Color = Color::Yellow;
}
