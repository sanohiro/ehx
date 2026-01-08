use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::{Action, EditMode, InputState, KeyMod, PrefixKey};
use crate::buffer::Document;
use crate::clipboard::{self, HexFormat};
use crate::encoding::{self, CharEncoding};
use crate::ui::{HexView, ViewMode};

/// アプリケーション状態
pub struct App {
    /// 編集中のドキュメント
    document: Document,
    /// カーソル位置
    cursor: usize,
    /// 表示オフセット
    offset: usize,
    /// 1行あたりのバイト数
    bytes_per_row: usize,
    /// 表示可能な行数
    visible_rows: usize,
    /// HEX/ASCIIモード
    hex_mode: bool,
    /// 編集モード
    edit_mode: EditMode,
    /// 入力状態
    input_state: InputState,
    /// プレフィックスキー状態（C-x等）
    prefix_key: PrefixKey,
    /// 選択範囲
    selection: Option<(usize, usize)>,
    /// 選択開始位置
    selection_start: Option<usize>,
    /// 文字エンコーディング
    encoding: CharEncoding,
    /// 終了フラグ
    should_quit: bool,
    /// ステータスメッセージ
    status_message: Option<String>,
    /// 検索モード
    search_mode: bool,
    /// 検索クエリ（入力中の文字列）
    search_query: String,
    /// 検索開始位置（検索キャンセル時に戻る位置）
    search_start_pos: usize,
}

impl App {
    /// 新しいアプリケーションを作成
    pub fn new() -> Self {
        Self {
            document: Document::new(),
            cursor: 0,
            offset: 0,
            bytes_per_row: 16,
            visible_rows: 24,
            hex_mode: true,
            edit_mode: EditMode::Overwrite,
            input_state: InputState::Normal,
            prefix_key: PrefixKey::None,
            selection: None,
            selection_start: None,
            encoding: CharEncoding::Utf8,
            should_quit: false,
            status_message: None,
            search_mode: false,
            search_query: String::new(),
            search_start_pos: 0,
        }
    }

    /// ファイルを開く
    pub fn open(&mut self, path: impl Into<PathBuf>) -> Result<()> {
        self.document = Document::open(path)?;
        self.cursor = 0;
        self.offset = 0;
        self.selection = None;
        Ok(())
    }

    /// 終了すべきかどうか
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// ファイル名を取得
    pub fn filename(&self) -> Option<&str> {
        self.document.filename()
    }

    /// 変更されているかどうか
    pub fn is_modified(&self) -> bool {
        self.document.is_modified()
    }

    /// 表示可能行数を設定
    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows.saturating_sub(4); // ヘッダー、ステータスバー分
    }

    /// カーソルを上に移動
    fn cursor_up(&mut self) {
        if self.cursor >= self.bytes_per_row {
            self.cursor -= self.bytes_per_row;
            self.ensure_cursor_visible();
        }
    }

    /// カーソルを下に移動
    fn cursor_down(&mut self) {
        let new_pos = self.cursor + self.bytes_per_row;
        if new_pos < self.document.len() {
            self.cursor = new_pos;
            self.ensure_cursor_visible();
        }
    }

    /// カーソルを左に移動
    fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.ensure_cursor_visible();
        }
    }

    /// カーソルを右に移動（EOF位置まで移動可能）
    fn cursor_right(&mut self) {
        if self.cursor < self.document.len() {
            self.cursor += 1;
            self.ensure_cursor_visible();
        }
    }

    /// カーソル位置が表示範囲内になるようにスクロール
    fn ensure_cursor_visible(&mut self) {
        let cursor_row = self.cursor / self.bytes_per_row;
        let offset_row = self.offset / self.bytes_per_row;

        if cursor_row < offset_row {
            self.offset = cursor_row * self.bytes_per_row;
        } else if cursor_row >= offset_row + self.visible_rows {
            self.offset = (cursor_row - self.visible_rows + 1) * self.bytes_per_row;
        }
    }

    /// ページアップ
    fn page_up(&mut self) {
        let page_size = self.visible_rows * self.bytes_per_row;
        self.cursor = self.cursor.saturating_sub(page_size);
        self.offset = self.offset.saturating_sub(page_size);
    }

    /// ページダウン
    fn page_down(&mut self) {
        let page_size = self.visible_rows * self.bytes_per_row;
        let max_pos = self.document.len(); // EOF位置まで移動可能
        self.cursor = (self.cursor + page_size).min(max_pos);
        self.offset = (self.offset + page_size).min(
            (self.document.len() / self.bytes_per_row).saturating_sub(self.visible_rows)
                * self.bytes_per_row,
        );
        self.ensure_cursor_visible();
    }

    /// 行頭に移動
    fn cursor_home(&mut self) {
        self.cursor = (self.cursor / self.bytes_per_row) * self.bytes_per_row;
    }

    /// 行末に移動（EOF位置まで移動可能）
    fn cursor_end(&mut self) {
        let row_start = (self.cursor / self.bytes_per_row) * self.bytes_per_row;
        let row_end = (row_start + self.bytes_per_row).min(self.document.len());
        self.cursor = row_end;
    }

    /// HEX入力処理
    fn input_hex(&mut self, ch: char) {
        // 全角→半角、小文字→大文字の正規化
        let normalized = Self::normalize_hex_char(ch);
        let Some(digit) = normalized.and_then(|c| c.to_digit(16)) else {
            return;
        };
        let digit = digit as u8;

        match self.input_state {
            InputState::Normal => {
                // 1桁目：上位ニブルを即座に反映
                match self.edit_mode {
                    EditMode::Overwrite => {
                        // 上書きモード：既存バイトの下位ニブルは保持
                        let low_nibble = if self.cursor < self.document.len() {
                            self.document.get(self.cursor).unwrap_or(0) & 0x0F
                        } else {
                            0
                        };
                        let value = (digit << 4) | low_nibble;
                        if self.cursor < self.document.len() {
                            let _ = self.document.set(self.cursor, value);
                        } else {
                            let _ = self.document.insert(self.cursor, value);
                        }
                    }
                    EditMode::Insert => {
                        // 挿入モード：新しいバイトを挿入
                        let value = digit << 4;
                        let _ = self.document.insert(self.cursor, value);
                    }
                }
                self.input_state = InputState::HexFirstDigit(digit);
            }
            InputState::HexFirstDigit(first) => {
                // 2桁目：下位ニブルを更新して次へ
                let value = (first << 4) | digit;
                // 1桁目で既にバイトが存在するので上書き
                let _ = self.document.set(self.cursor, value);
                self.cursor_right();
                self.input_state = InputState::Normal;
            }
        }
    }

    /// HEX文字の正規化（全角→半角、小文字→大文字）
    /// 0-9, A-F以外はNoneを返す
    fn normalize_hex_char(ch: char) -> Option<char> {
        match ch {
            // 半角数字
            '0'..='9' => Some(ch),
            // 半角英字（大文字）
            'A'..='F' => Some(ch),
            // 半角英字（小文字）→ 大文字に変換
            'a'..='f' => Some(ch.to_ascii_uppercase()),
            // 全角数字 → 半角に変換
            '０' => Some('0'),
            '１' => Some('1'),
            '２' => Some('2'),
            '３' => Some('3'),
            '４' => Some('4'),
            '５' => Some('5'),
            '６' => Some('6'),
            '７' => Some('7'),
            '８' => Some('8'),
            '９' => Some('9'),
            // 全角英字（大文字）→ 半角に変換
            'Ａ' => Some('A'),
            'Ｂ' => Some('B'),
            'Ｃ' => Some('C'),
            'Ｄ' => Some('D'),
            'Ｅ' => Some('E'),
            'Ｆ' => Some('F'),
            // 全角英字（小文字）→ 半角大文字に変換
            'ａ' => Some('A'),
            'ｂ' => Some('B'),
            'ｃ' => Some('C'),
            'ｄ' => Some('D'),
            'ｅ' => Some('E'),
            'ｆ' => Some('F'),
            // それ以外は無効
            _ => None,
        }
    }

    /// ASCII入力処理（文字をバッファのエンコーディングに変換して入力）
    fn input_ascii(&mut self, ch: char) {
        // 文字をバッファのエンコーディングに変換
        let bytes = match encoding::encode_char(ch, self.encoding) {
            Some(bytes) => bytes,
            None => {
                // エンコードできない文字
                self.status_message = Some(format!(
                    "Cannot encode '{}' in {}",
                    ch,
                    self.encoding.name()
                ));
                return;
            }
        };

        if bytes.is_empty() {
            return;
        }

        match self.edit_mode {
            EditMode::Overwrite => {
                // 上書きモード：各バイトを順番に上書き（EOFを超えた分は追加）
                for (i, &byte) in bytes.iter().enumerate() {
                    let pos = self.cursor + i;
                    if pos < self.document.len() {
                        let _ = self.document.set(pos, byte);
                    } else {
                        let _ = self.document.insert(pos, byte);
                    }
                }
            }
            EditMode::Insert => {
                // 挿入モード：バイト列を挿入
                for (i, &byte) in bytes.iter().enumerate() {
                    let _ = self.document.insert(self.cursor + i, byte);
                }
            }
        }

        // カーソルをバイト数分進める
        for _ in 0..bytes.len() {
            self.cursor_right();
        }
    }

    /// 選択開始（マークを設定）
    fn start_selection(&mut self) {
        self.selection_start = Some(self.cursor);
        self.selection = Some((self.cursor, self.cursor));
        self.status_message = Some("Mark set".to_string());
    }

    /// 選択解除
    fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection = None;
    }

    /// 選択範囲を更新（カーソル移動後に呼ぶ）
    fn update_selection(&mut self) {
        if let Some(start) = self.selection_start {
            let (sel_start, sel_end) = if start <= self.cursor {
                (start, self.cursor)
            } else {
                (self.cursor, start)
            };
            self.selection = Some((sel_start, sel_end));
        }
    }

    /// 選択しながら上に移動
    fn select_up(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        }
        self.cursor_up();
        self.update_selection();
    }

    /// 選択しながら下に移動
    fn select_down(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        }
        self.cursor_down();
        self.update_selection();
    }

    /// 選択しながら左に移動
    fn select_left(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        }
        self.cursor_left();
        self.update_selection();
    }

    /// 選択しながら右に移動
    fn select_right(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        }
        self.cursor_right();
        self.update_selection();
    }

    /// 選択範囲をコピー (M-w)
    /// システムクリップボード + OSC 52 (ターミナルクリップボード)
    fn copy(&mut self) {
        if let Some((start, end)) = self.selection {
            if let Some(data) = self.document.get_range(start, end + 1) {
                // 両方のクリップボードにコピー
                let _ = clipboard::copy_hex_to_all(data, HexFormat::Spaced);
                self.status_message = Some(format!("Copied {} bytes", end - start + 1));
                self.clear_selection();
            }
        } else {
            self.status_message = Some("No selection".to_string());
        }
    }

    /// 選択範囲をHEX形式でコピー
    fn copy_hex(&mut self) {
        if let Some((start, end)) = self.selection {
            if let Some(data) = self.document.get_range(start, end + 1) {
                // 両方のクリップボードにコピー
                let _ = clipboard::copy_hex_to_all(data, HexFormat::Spaced);
                self.status_message = Some("Copied as HEX".to_string());
                self.clear_selection();
            }
        } else if let Some(byte) = self.document.get(self.cursor) {
            let _ = clipboard::copy_hex_to_all(&[byte], HexFormat::Spaced);
        }
    }

    /// 選択範囲をカット (C-w)
    /// システムクリップボード + OSC 52 (ターミナルクリップボード)
    fn cut(&mut self) {
        if let Some((start, end)) = self.selection {
            if let Some(data) = self.document.get_range(start, end + 1) {
                // 両方のクリップボードにコピー
                let _ = clipboard::copy_hex_to_all(data, HexFormat::Spaced);
                // 選択範囲を削除（末尾から削除）
                for i in (start..=end).rev() {
                    let _ = self.document.delete(i);
                }
                self.cursor = start;
                self.status_message = Some(format!("Cut {} bytes", end - start + 1));
                self.clear_selection();
            }
        } else {
            self.status_message = Some("No selection".to_string());
        }
    }

    /// システムクリップボードからペースト (C-y)
    fn paste(&mut self) {
        // システムクリップボードからテキストを取得
        let content = match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
            Ok(text) => text,
            Err(_) => {
                self.status_message = Some("Clipboard empty or unavailable".to_string());
                return;
            }
        };
        self.paste_from_terminal(&content);
    }

    /// ターミナルからのペースト（Bracketed Paste）を処理
    /// ペーストされた内容をバイト列としてカーソル位置に挿入
    fn paste_from_terminal(&mut self, content: &str) {
        // HEX文字列かどうかを判定（全角文字も正規化して判定）
        let trimmed = content.trim();
        let bytes = if Self::looks_like_hex(trimmed) {
            // HEX文字列として解釈（全角→半角、小文字→大文字も変換）
            Self::normalized_hex_to_bytes(trimmed).unwrap_or_else(|| content.as_bytes().to_vec())
        } else {
            // 生のバイト列として扱う
            content.as_bytes().to_vec()
        };

        if bytes.is_empty() {
            return;
        }

        // 選択範囲があれば削除してから挿入
        if let Some((start, end)) = self.selection {
            for i in (start..=end).rev() {
                let _ = self.document.delete(i);
            }
            self.cursor = start;
            self.clear_selection();
        }

        // 編集モードに応じて処理
        match self.edit_mode {
            EditMode::Overwrite => {
                // 上書きモード：既存バイトを上書き、EOFを超えた分は追加
                for (i, &byte) in bytes.iter().enumerate() {
                    let pos = self.cursor + i;
                    if pos < self.document.len() {
                        let _ = self.document.set(pos, byte);
                    } else {
                        let _ = self.document.insert(pos, byte);
                    }
                }
            }
            EditMode::Insert => {
                // 挿入モード：カーソル位置にバイト列を挿入
                for (i, &byte) in bytes.iter().enumerate() {
                    let _ = self.document.insert(self.cursor + i, byte);
                }
            }
        }

        self.cursor += bytes.len();
        self.ensure_cursor_visible();
        self.status_message = Some(format!("Pasted {} bytes", bytes.len()));
    }

    /// 検索クエリをバイト列に変換
    fn search_query_to_bytes(&self) -> Vec<u8> {
        let trimmed = self.search_query.trim();
        if Self::looks_like_hex(trimmed) {
            Self::normalized_hex_to_bytes(trimmed).unwrap_or_else(|| self.search_query.as_bytes().to_vec())
        } else {
            self.search_query.as_bytes().to_vec()
        }
    }

    /// 前方検索（現在位置から後ろへ）
    fn find_next(&mut self) {
        let pattern = self.search_query_to_bytes();
        if pattern.is_empty() {
            return;
        }

        let data = self.document.data();
        let start = self.cursor + 1;

        // 現在位置から末尾まで検索
        if let Some(pos) = Self::find_pattern(data, &pattern, start) {
            self.cursor = pos;
            self.ensure_cursor_visible();
            self.status_message = Some(format!("Found at {:08X}", pos));
            return;
        }

        // 先頭から現在位置まで検索（ラップアラウンド）
        if let Some(pos) = Self::find_pattern(data, &pattern, 0) {
            if pos < start {
                self.cursor = pos;
                self.ensure_cursor_visible();
                self.status_message = Some(format!("Wrapped, found at {:08X}", pos));
                return;
            }
        }

        self.status_message = Some("Not found".to_string());
    }

    /// 後方検索（現在位置から前へ）
    fn find_prev(&mut self) {
        let pattern = self.search_query_to_bytes();
        if pattern.is_empty() {
            return;
        }

        let data = self.document.data();
        let end = self.cursor;

        // 現在位置から先頭まで検索
        if let Some(pos) = Self::find_pattern_reverse(data, &pattern, end) {
            self.cursor = pos;
            self.ensure_cursor_visible();
            self.status_message = Some(format!("Found at {:08X}", pos));
            return;
        }

        // 末尾から現在位置まで検索（ラップアラウンド）
        if let Some(pos) = Self::find_pattern_reverse(data, &pattern, data.len()) {
            if pos > end {
                self.cursor = pos;
                self.ensure_cursor_visible();
                self.status_message = Some(format!("Wrapped, found at {:08X}", pos));
                return;
            }
        }

        self.status_message = Some("Not found".to_string());
    }

    /// パターンを前方検索
    fn find_pattern(data: &[u8], pattern: &[u8], start: usize) -> Option<usize> {
        if pattern.is_empty() || start + pattern.len() > data.len() {
            return None;
        }
        data[start..].windows(pattern.len()).position(|w| w == pattern).map(|p| p + start)
    }

    /// パターンを後方検索
    fn find_pattern_reverse(data: &[u8], pattern: &[u8], end: usize) -> Option<usize> {
        if pattern.is_empty() || end == 0 {
            return None;
        }
        let search_end = end.min(data.len());
        if search_end < pattern.len() {
            return None;
        }
        data[..search_end].windows(pattern.len()).rposition(|w| w == pattern)
    }

    /// 文字列がHEX形式かどうかを判定（全角文字も考慮）
    fn looks_like_hex(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        // 正規化してからチェック
        let normalized = Self::normalize_hex_string(s);

        // 偶数長で全て16進数なら HEX とみなす
        normalized.len() % 2 == 0
            && normalized.len() >= 2
            && normalized.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// HEX文字列を正規化（全角→半角、小文字→大文字、区切り文字除去）
    fn normalize_hex_string(s: &str) -> String {
        s.chars()
            .filter_map(|c| {
                // 区切り文字をスキップ
                if c == ' ' || c == ',' || c == '{' || c == '}' || c == '\n' || c == '\r' || c == '\t' {
                    return None;
                }
                // 0x プレフィックスをスキップ
                if c == 'x' || c == 'X' || c == 'ｘ' || c == 'Ｘ' {
                    return None;
                }
                // 正規化
                Self::normalize_hex_char(c)
            })
            .collect()
    }

    /// 正規化されたHEX文字列をバイト列に変換
    fn normalized_hex_to_bytes(s: &str) -> Option<Vec<u8>> {
        let normalized = Self::normalize_hex_string(s);
        if normalized.len() % 2 != 0 {
            return None;
        }
        let mut bytes = Vec::with_capacity(normalized.len() / 2);
        let chars: Vec<char> = normalized.chars().collect();
        for i in (0..chars.len()).step_by(2) {
            let high = chars[i].to_digit(16)?;
            let low = chars[i + 1].to_digit(16)?;
            bytes.push(((high << 4) | low) as u8);
        }
        Some(bytes)
    }

    /// アクションを実行
    pub fn execute(&mut self, action: Action) {
        // ステータスメッセージをクリア（一部のアクションを除く）
        if !matches!(action, Action::EnterCtrlX) {
            self.status_message = None;
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::Save => {
                if let Err(e) = self.document.save() {
                    self.status_message = Some(format!("Save failed: {}", e));
                } else {
                    self.status_message = Some("Saved".to_string());
                }
            }
            // カーソル移動（選択開始中は選択範囲を更新）
            Action::CursorUp => {
                self.cursor_up();
                self.update_selection();
            }
            Action::CursorDown => {
                self.cursor_down();
                self.update_selection();
            }
            Action::CursorLeft => {
                self.cursor_left();
                self.update_selection();
            }
            Action::CursorRight => {
                self.cursor_right();
                self.update_selection();
            }
            Action::CursorHome => {
                self.cursor_home();
                self.update_selection();
            }
            Action::CursorEnd => {
                self.cursor_end();
                self.update_selection();
            }
            Action::PageUp => {
                self.page_up();
                self.update_selection();
            }
            Action::PageDown => {
                self.page_down();
                self.update_selection();
            }
            Action::GotoBeginning => {
                self.cursor = 0;
                self.offset = 0;
                self.update_selection();
            }
            Action::GotoEnd => {
                self.cursor = self.document.len(); // EOF位置
                self.ensure_cursor_visible();
                self.update_selection();
            }
            // 選択操作
            Action::StartSelection => self.start_selection(),
            Action::ClearSelection => self.clear_selection(),
            Action::SelectUp => self.select_up(),
            Action::SelectDown => self.select_down(),
            Action::SelectLeft => self.select_left(),
            Action::SelectRight => self.select_right(),
            // クリップボード
            Action::Copy => self.copy(),
            Action::CopyHex => self.copy_hex(),
            Action::Cut => self.cut(),
            Action::Paste => self.paste(),
            // モード切替
            Action::ToggleMode => self.hex_mode = !self.hex_mode,
            Action::ToggleEditMode => {
                self.edit_mode = match self.edit_mode {
                    EditMode::Overwrite => EditMode::Insert,
                    EditMode::Insert => EditMode::Overwrite,
                };
            }
            Action::ToggleEncoding => {
                self.encoding = self.encoding.next();
                self.status_message = Some(format!("Encoding: {}", self.encoding.name()));
            }
            // 入力
            Action::InputHex(ch) => self.input_hex(ch),
            Action::InputAscii(ch) => self.input_ascii(ch),
            // プレフィックスキー
            Action::EnterCtrlX => {
                self.prefix_key = PrefixKey::CtrlX;
                self.status_message = Some("C-x-".to_string());
            }
            Action::Cancel => {
                self.prefix_key = PrefixKey::None;
                self.input_state = InputState::Normal;
                self.clear_selection();
                self.status_message = Some("Quit".to_string());
            }
            // Undo/Redo
            Action::Undo => {
                if let Some(pos) = self.document.undo() {
                    self.cursor = pos.min(self.document.len().saturating_sub(1));
                    self.ensure_cursor_visible();
                    self.status_message = Some("Undo".to_string());
                } else {
                    self.status_message = Some("Nothing to undo".to_string());
                }
            }
            Action::Redo => {
                if let Some(pos) = self.document.redo() {
                    self.cursor = pos.min(self.document.len().saturating_sub(1));
                    self.ensure_cursor_visible();
                    self.status_message = Some("Redo".to_string());
                } else {
                    self.status_message = Some("Nothing to redo".to_string());
                }
            }
            // 検索
            Action::StartSearch => {
                self.search_mode = true;
                self.search_query.clear();
                self.search_start_pos = self.cursor;
            }
            Action::StartSearchBack => {
                self.search_mode = true;
                self.search_query.clear();
                self.search_start_pos = self.cursor;
            }
            Action::SearchNext => {
                if !self.search_query.is_empty() {
                    self.find_next();
                }
            }
            Action::SearchPrev => {
                if !self.search_query.is_empty() {
                    self.find_prev();
                }
            }
            _ => {}
        }
    }

    /// イベントを処理
    pub fn handle_event(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                // ペーストイベント（Bracketed Paste Mode）
                Event::Paste(content) => {
                    if self.search_mode {
                        // 検索モード中はクエリに追加
                        self.search_query.push_str(&content);
                        self.do_incremental_search();
                    } else {
                        self.paste_from_terminal(&content);
                    }
                }
                // キーイベント
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    // 検索モード中は特別な処理
                    if self.search_mode {
                        self.handle_search_key(key);
                        return Ok(());
                    }

                    let mods = KeyMod {
                        ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                        shift: key.modifiers.contains(KeyModifiers::SHIFT),
                        alt: key.modifiers.contains(KeyModifiers::ALT),
                    };

                    // プレフィックスキー状態に応じて処理を分岐
                    let action = match self.prefix_key {
                        PrefixKey::None => Action::from_key(key.code, mods),
                        PrefixKey::CtrlX => {
                            self.prefix_key = PrefixKey::None; // プレフィックス状態をリセット
                            Action::from_key_after_ctrl_x(key.code, mods)
                        }
                    };

                    if action != Action::None {
                        self.execute(action);
                    } else if let KeyCode::Char(ch) = key.code {
                        // 修飾キーがなければ文字入力
                        if !mods.ctrl && !mods.alt {
                            if self.hex_mode {
                                self.execute(Action::InputHex(ch));
                            } else {
                                self.execute(Action::InputAscii(ch));
                            }
                        }
                    }
                }
                // フォーカスイベント
                Event::FocusGained => {
                    // フォーカス復帰時：将来的にファイルの外部変更チェックを行う
                    self.status_message = Some("Focus gained".to_string());
                }
                Event::FocusLost => {
                    // フォーカス喪失時：特に何もしない
                }
                // その他のイベントは無視
                _ => {}
            }
        }
        Ok(())
    }

    /// 検索モード中のキー処理
    fn handle_search_key(&mut self, key: crossterm::event::KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Escape / C-g: 検索キャンセル
            KeyCode::Esc | KeyCode::Char('g') if ctrl => {
                self.search_mode = false;
                self.cursor = self.search_start_pos;
                self.ensure_cursor_visible();
                self.status_message = Some("Cancelled".to_string());
            }
            // Enter: 検索確定
            KeyCode::Enter => {
                self.search_mode = false;
                if self.search_query.is_empty() {
                    self.status_message = Some("Search cancelled".to_string());
                } else {
                    self.status_message = Some(format!("I-search: {}", self.search_query));
                }
            }
            // C-s: 次を検索
            KeyCode::Char('s') if ctrl => {
                self.find_next();
            }
            // C-r: 前を検索
            KeyCode::Char('r') if ctrl => {
                self.find_prev();
            }
            // Backspace: 1文字削除
            KeyCode::Backspace => {
                self.search_query.pop();
                if self.search_query.is_empty() {
                    self.cursor = self.search_start_pos;
                    self.ensure_cursor_visible();
                } else {
                    self.do_incremental_search();
                }
            }
            // 文字入力
            KeyCode::Char(ch) if !ctrl => {
                self.search_query.push(ch);
                self.do_incremental_search();
            }
            _ => {}
        }
    }

    /// インクリメンタル検索を実行
    fn do_incremental_search(&mut self) {
        let pattern = self.search_query_to_bytes();
        if pattern.is_empty() {
            return;
        }

        let data = self.document.data();
        // 検索開始位置から検索
        if let Some(pos) = Self::find_pattern(data, &pattern, self.search_start_pos) {
            self.cursor = pos;
            self.ensure_cursor_visible();
        } else if let Some(pos) = Self::find_pattern(data, &pattern, 0) {
            // ラップアラウンド
            self.cursor = pos;
            self.ensure_cursor_visible();
        }
    }

    /// UIを描画
    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();
        self.set_visible_rows(size.height as usize);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // タイトル
                Constraint::Min(1),    // メイン
                Constraint::Length(1), // ステータス
            ])
            .split(size);

        // タイトルバー
        let title = format!(
            " hx - {} {}",
            self.document
                .filename()
                .unwrap_or("[New File]"),
            if self.document.is_modified() { "[+]" } else { "" }
        );
        let title_widget = Paragraph::new(title)
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        frame.render_widget(title_widget, layout[0]);

        // HEXビュー
        let hex_view = HexView::new(self.document.data())
            .offset(self.offset)
            .cursor(self.cursor)
            .selection(self.selection)
            .bytes_per_row(self.bytes_per_row)
            .encoding(self.encoding)
            .mode(if self.hex_mode {
                ViewMode::Hex
            } else {
                ViewMode::Ascii
            });
        frame.render_widget(hex_view, layout[1]);

        // ステータスバー
        let mode_str = if self.hex_mode { "HEX" } else { "ASCII" };
        let edit_str = match self.edit_mode {
            EditMode::Overwrite => "OVR",
            EditMode::Insert => "INS",
        };

        let status = if self.search_mode {
            // 検索モード中は検索プロンプトを表示
            format!("I-search: {}_", self.search_query)
        } else if let Some(ref msg) = self.status_message {
            msg.clone()
        } else {
            let byte_info = if self.cursor < self.document.len() {
                let byte = self.document.get(self.cursor).unwrap_or(0);
                format!("Byte: {:02X} ({:3})", byte, byte)
            } else {
                "EOF".to_string()
            };
            format!(
                " {:08X} / {:08X} | {} | {} | {} | {}",
                self.cursor,
                self.document.len(),
                mode_str,
                edit_str,
                self.encoding.name(),
                byte_info,
            )
        };

        let status_widget = Paragraph::new(status)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(status_widget, layout[2]);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
