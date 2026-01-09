mod state;

pub use state::App;

use crossterm::event::KeyCode;

/// 編集モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditMode {
    #[default]
    Overwrite,
    Insert,
}

/// 入力状態（HEX入力は2桁で1バイト）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputState {
    #[default]
    Normal,
    /// HEX入力の1桁目を入力済み
    HexFirstDigit(u8),
}

/// プレフィックスキー状態（Emacs 2ストローク用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefixKey {
    #[default]
    None,
    /// C-x を押した状態
    CtrlX,
}

/// アプリケーションアクション
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Action {
    Quit,
    Save,

    // カーソル移動
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    PageUp,
    PageDown,
    GotoBeginning,  // M-< バッファ先頭
    GotoEnd,        // M-> バッファ末尾（EOF）
    GotoAddress(usize),

    // 編集
    InputHex(char),
    InputAscii(char),
    Delete,
    Backspace,
    ToggleMode,         // HEX <-> ASCII
    ToggleEditMode,     // Insert <-> Overwrite

    // 選択
    StartSelection,
    ClearSelection,
    SelectAll,
    // Shift+矢印キーによる選択
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,

    // クリップボード
    Copy,       // M-w: コピー
    CopyHex,    // HEX形式でコピー
    Cut,        // C-w: カット (kill-region)
    Paste,      // C-y: ペースト
    PasteHex,

    // 表示
    ToggleEncoding,
    SetBytesPerRow(usize),

    // 検索
    StartSearch,     // C-s: 検索モード開始
    StartSearchBack, // C-r: 後方検索モード開始
    Search(Vec<u8>),
    SearchNext,
    SearchPrev,

    // 置換
    StartReplace,    // M-%: query-replace開始

    // その他
    Undo,
    Redo,

    // プレフィックスキー
    EnterCtrlX,  // C-x を押した
    Cancel,      // C-g でキャンセル

    // コマンド
    ExecuteCommand,  // M-x: コマンド実行

    // ジャンプ・ファイル操作
    StartGoto,   // M-g: アドレスジャンプ
    OpenFile,    // C-x C-f: ファイルを開く
    SaveAs,      // C-x C-w: 別名保存
    KillBuffer,  // C-x k: バッファを閉じる

    None,
}

/// キー修飾子
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyMod {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Action {
    /// キーコードからアクションに変換（Emacsキーバインド）
    pub fn from_key(key: KeyCode, mods: KeyMod) -> Self {
        let KeyMod { ctrl, shift, alt } = mods;

        match (key, ctrl, alt, shift) {
            // === Emacsプレフィックスキー ===
            // C-x: プレフィックスキーモードへ
            (KeyCode::Char('x'), true, false, false) => Action::EnterCtrlX,

            // C-g: キャンセル
            (KeyCode::Char('g'), true, false, false) => Action::Cancel,
            (KeyCode::Esc, _, _, _) => Action::Cancel,

            // === Emacsカーソル移動 ===
            // Ctrl+F: 右
            (KeyCode::Char('f'), true, false, false) => Action::CursorRight,
            // Ctrl+B: 左
            (KeyCode::Char('b'), true, false, false) => Action::CursorLeft,
            // Ctrl+N: 下
            (KeyCode::Char('n'), true, false, false) => Action::CursorDown,
            // Ctrl+P: 上
            (KeyCode::Char('p'), true, false, false) => Action::CursorUp,
            // Ctrl+A: 行頭
            (KeyCode::Char('a'), true, false, false) => Action::CursorHome,
            // Ctrl+E: 行末
            (KeyCode::Char('e'), true, false, false) => Action::CursorEnd,
            // Ctrl+V: ページダウン
            (KeyCode::Char('v'), true, false, false) => Action::PageDown,
            // Alt+V: ページアップ
            (KeyCode::Char('v'), false, true, false) => Action::PageUp,
            // M-< : バッファ先頭
            (KeyCode::Char('<'), false, true, _) => Action::GotoBeginning,
            // M-> : バッファ末尾（EOF）
            (KeyCode::Char('>'), false, true, _) => Action::GotoEnd,

            // 矢印キー（修飾キーなし）
            (KeyCode::Up, false, false, false) => Action::CursorUp,
            (KeyCode::Down, false, false, false) => Action::CursorDown,
            (KeyCode::Left, false, false, false) => Action::CursorLeft,
            (KeyCode::Right, false, false, false) => Action::CursorRight,
            // Shift+矢印キー: 選択
            (KeyCode::Up, false, false, true) => Action::SelectUp,
            (KeyCode::Down, false, false, true) => Action::SelectDown,
            (KeyCode::Left, false, false, true) => Action::SelectLeft,
            (KeyCode::Right, false, false, true) => Action::SelectRight,
            (KeyCode::Home, _, _, _) => Action::CursorHome,
            (KeyCode::End, _, _, _) => Action::CursorEnd,
            (KeyCode::PageUp, _, _, _) => Action::PageUp,
            (KeyCode::PageDown, _, _, _) => Action::PageDown,

            // モード切替
            (KeyCode::Tab, false, false, _) => Action::ToggleMode,
            (KeyCode::Insert, false, false, _) => Action::ToggleEditMode,

            // === Emacs編集 ===
            // Ctrl+D: 削除（カーソル位置）
            (KeyCode::Char('d'), true, false, false) => Action::Delete,
            (KeyCode::Delete, false, false, _) => Action::Delete,
            (KeyCode::Backspace, false, false, _) => Action::Backspace,

            // === Emacsクリップボード ===
            // Ctrl+Space: 選択開始（マーク設定）
            (KeyCode::Char(' '), true, false, false) => Action::StartSelection,
            // C-w: カット (kill-region)
            (KeyCode::Char('w'), true, false, false) => Action::Cut,
            // M-w: コピー (kill-ring-save)
            (KeyCode::Char('w'), false, true, false) => Action::Copy,
            // Ctrl+Y: ペースト (yank)
            (KeyCode::Char('y'), true, false, false) => Action::Paste,

            // Undo: C-u (ze style)
            (KeyCode::Char('u'), true, false, false) => Action::Undo,
            // Redo: C-/ (ze style)
            (KeyCode::Char('/'), true, false, false) => Action::Redo,

            // 検索: C-s (前方), C-r (後方)
            (KeyCode::Char('s'), true, false, false) => Action::StartSearch,
            (KeyCode::Char('r'), true, false, false) => Action::StartSearchBack,

            // 置換: M-% (query-replace)
            (KeyCode::Char('%'), false, true, _) => Action::StartReplace,

            // コマンド: M-x
            (KeyCode::Char('x'), false, true, false) => Action::ExecuteCommand,

            // ジャンプ: M-g (goto-address)
            (KeyCode::Char('g'), false, true, false) => Action::StartGoto,

            // エンコーディング切替: F2
            (KeyCode::F(2), false, false, _) => Action::ToggleEncoding,

            _ => Action::None,
        }
    }

    /// C-x の後のキーを処理
    pub fn from_key_after_ctrl_x(key: KeyCode, mods: KeyMod) -> Self {
        let KeyMod { ctrl, .. } = mods;

        match (key, ctrl) {
            // C-x C-c: 終了
            (KeyCode::Char('c'), true) => Action::Quit,
            // C-x C-s: 保存
            (KeyCode::Char('s'), true) => Action::Save,
            // C-x C-f: ファイルを開く
            (KeyCode::Char('f'), true) => Action::OpenFile,
            // C-x C-w: 別名保存
            (KeyCode::Char('w'), true) => Action::SaveAs,
            // C-x k: バッファを閉じる
            (KeyCode::Char('k'), false) => Action::KillBuffer,

            // C-g: キャンセル
            (KeyCode::Char('g'), true) => Action::Cancel,
            (KeyCode::Esc, _) => Action::Cancel,

            // その他は無効
            _ => Action::Cancel,
        }
    }
}
