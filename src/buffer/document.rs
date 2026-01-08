use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::BufferError;

/// Undo/Redo用の操作記録
#[derive(Debug, Clone)]
enum UndoOp {
    /// バイトの上書き (位置, 旧値, 新値)
    Set(usize, u8, u8),
    /// バイトの挿入 (位置, 値)
    Insert(usize, u8),
    /// バイトの削除 (位置, 値)
    Delete(usize, u8),
}

/// バイナリドキュメントを表す構造体
pub struct Document {
    /// ファイルパス
    path: Option<PathBuf>,
    /// バッファデータ
    data: Vec<u8>,
    /// 変更フラグ
    modified: bool,
    /// 読み取り専用フラグ
    readonly: bool,
    /// Undo履歴
    undo_stack: Vec<UndoOp>,
    /// Redo履歴
    redo_stack: Vec<UndoOp>,
}

impl Document {
    /// 空のドキュメントを作成
    pub fn new() -> Self {
        Self {
            path: None,
            data: Vec::new(),
            modified: false,
            readonly: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// ファイルから読み込み
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, BufferError> {
        let path = path.into();
        let mut file = File::open(&path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        Ok(Self {
            path: Some(path),
            data,
            modified: false,
            readonly: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    /// ファイルに保存
    pub fn save(&mut self) -> Result<(), BufferError> {
        if let Some(ref path) = self.path {
            let mut file = File::create(path)?;
            file.write_all(&self.data)?;
            self.modified = false;
            Ok(())
        } else {
            Err(BufferError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No file path set",
            )))
        }
    }

    /// 別名で保存
    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), BufferError> {
        self.path = Some(path.into());
        self.save()
    }

    /// データの長さを取得
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// データが空かどうか
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 指定位置のバイトを取得
    pub fn get(&self, pos: usize) -> Option<u8> {
        self.data.get(pos).copied()
    }

    /// 指定範囲のバイト列を取得
    pub fn get_range(&self, start: usize, end: usize) -> Option<&[u8]> {
        if start <= end && end <= self.data.len() {
            Some(&self.data[start..end])
        } else {
            None
        }
    }

    /// 指定位置のバイトを設定
    pub fn set(&mut self, pos: usize, value: u8) -> Result<(), BufferError> {
        if pos < self.data.len() {
            let old_value = self.data[pos];
            if old_value != value {
                self.data[pos] = value;
                self.modified = true;
                self.undo_stack.push(UndoOp::Set(pos, old_value, value));
                self.redo_stack.clear();
            }
            Ok(())
        } else {
            Err(BufferError::OutOfBounds(pos))
        }
    }

    /// 指定位置にバイトを挿入
    pub fn insert(&mut self, pos: usize, value: u8) -> Result<(), BufferError> {
        if pos <= self.data.len() {
            self.data.insert(pos, value);
            self.modified = true;
            self.undo_stack.push(UndoOp::Insert(pos, value));
            self.redo_stack.clear();
            Ok(())
        } else {
            Err(BufferError::OutOfBounds(pos))
        }
    }

    /// 指定位置のバイトを削除
    pub fn delete(&mut self, pos: usize) -> Result<u8, BufferError> {
        if pos < self.data.len() {
            let value = self.data.remove(pos);
            self.modified = true;
            self.undo_stack.push(UndoOp::Delete(pos, value));
            self.redo_stack.clear();
            Ok(value)
        } else {
            Err(BufferError::OutOfBounds(pos))
        }
    }

    /// Undo: 直前の操作を取り消す
    /// 戻り値: (成功したか, 影響を受けた位置)
    pub fn undo(&mut self) -> Option<usize> {
        let op = self.undo_stack.pop()?;
        let pos = match op {
            UndoOp::Set(pos, old_value, new_value) => {
                self.data[pos] = old_value;
                self.redo_stack.push(UndoOp::Set(pos, old_value, new_value));
                pos
            }
            UndoOp::Insert(pos, value) => {
                self.data.remove(pos);
                self.redo_stack.push(UndoOp::Insert(pos, value));
                pos.saturating_sub(1).min(self.data.len().saturating_sub(1))
            }
            UndoOp::Delete(pos, value) => {
                self.data.insert(pos, value);
                self.redo_stack.push(UndoOp::Delete(pos, value));
                pos
            }
        };
        self.modified = !self.undo_stack.is_empty();
        Some(pos)
    }

    /// Redo: 取り消した操作をやり直す
    /// 戻り値: (成功したか, 影響を受けた位置)
    pub fn redo(&mut self) -> Option<usize> {
        let op = self.redo_stack.pop()?;
        let pos = match op {
            UndoOp::Set(pos, old_value, new_value) => {
                self.data[pos] = new_value;
                self.undo_stack.push(UndoOp::Set(pos, old_value, new_value));
                pos
            }
            UndoOp::Insert(pos, value) => {
                self.data.insert(pos, value);
                self.undo_stack.push(UndoOp::Insert(pos, value));
                pos
            }
            UndoOp::Delete(pos, value) => {
                self.data.remove(pos);
                self.undo_stack.push(UndoOp::Delete(pos, value));
                pos.min(self.data.len().saturating_sub(1))
            }
        };
        self.modified = true;
        Some(pos)
    }

    /// 変更されているかどうか
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// 読み取り専用かどうか
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// 読み取り専用フラグを設定
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }

    /// ファイルパスを取得
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// ファイル名を取得
    pub fn filename(&self) -> Option<&str> {
        self.path.as_ref().and_then(|p| p.file_name()).and_then(|s| s.to_str())
    }

    /// 生データへの参照を取得
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
