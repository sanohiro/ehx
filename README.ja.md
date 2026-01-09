# hx — Emacsキーバインドのバイナリエディタ

`C-x C-s` で考える人のためのターミナルバイナリエディタ。

このパッケージには2つのツールが含まれています:
- **hx** — 対話的TUIバイナリエディタ
- **bx** — パイプ用CLIバイナリツール

> xxdは原始的すぎる、GUIバイナリエディタは重すぎる。
> hxはその中間。

[English](README.md)

---

## なぜhx？

世の中のバイナリエディタはvi風かオリジナルのキーバインドばかり。**hx** はEmacsユーザーのためのバイナリエディタです。バイナリ編集のためだけに指の記憶を切り替える必要はありません。

- 同じ移動: `C-f`, `C-b`, `C-n`, `C-p`
- 同じファイル操作: `C-x C-s`, `C-x C-c`
- 同じ選択: `C-SPC`, `M-w`, `C-w`, `C-y`
- 同じ検索: `C-s`, `C-r`

覚え直し不要。モード切替の混乱なし。
テキストを編集するようにバイナリを編集。

---

## 特徴

- **Emacsキーバインド**
  おなじみのカーソル移動と編集コマンド。

- **デュアルペイン表示**
  HEXとASCIIを並列表示。Tabで入力フォーカスを切り替え。

- **マルチエンコーディング**
  UTF-8, UTF-16, Shift-JIS, EUC-JP など。

- **クリップボード連携**
  システムクリップボード + OSC 52（SSH越しでも動作）。

- **全角文字対応**
  `０-９`, `Ａ-Ｆ` もHEX入力として認識。日本語IMEフレンドリー。

---

## インストール

### Homebrew (macOS / Linux)

```bash
brew tap sanohiro/hx
brew install hx
```

### apt (Debian / Ubuntu)

```bash
curl -fsSL https://sanohiro.github.io/hx/hx.gpg | sudo gpg --dearmor -o /usr/share/keyrings/hx.gpg
echo "deb [signed-by=/usr/share/keyrings/hx.gpg] https://sanohiro.github.io/hx stable main" | sudo tee /etc/apt/sources.list.d/hx.list
sudo apt update
sudo apt install hx
```

### ソースからビルド

```bash
# Rust 1.85+ が必要
git clone https://github.com/sanohiro/hx
cd hx
cargo build --release
cp ./target/release/hx ./target/release/bx ~/.local/bin/
```

---

## クイックスタート

```bash
hx file.bin          # ファイルを開く
hx                   # 空のバッファで起動
cat file.bin | hx    # 標準入力から読み込み
echo -n "Hello" | hx # パイプでデータを渡す
```

保存して終了: `C-x C-s` → `C-x C-c`

---

## キーバインド

hxはEmacsスタイルのキーバインドを使用。`C-` はCtrl、`M-` はAlt/Option。

### カーソル移動

| キー | 動作 |
|------|------|
| `C-f` / `C-b` / `C-n` / `C-p` | カーソル移動 |
| `C-a` / `C-e` | 行頭 / 行末 |
| `C-v` / `M-v` | ページダウン / アップ |
| `M-<` / `M->` | バッファ先頭 / 末尾 |

### 編集

| キー | 動作 |
|------|------|
| `C-d` / `Backspace` | バイト削除 |
| `Tab` | HEX / ASCII入力切替 |
| `Insert` | 上書き / 挿入モード切替 |
| `C-u` / `C-/` | Undo / Redo |

### 選択とクリップボード

| キー | 動作 |
|------|------|
| `C-SPC` | 選択開始 |
| `M-w` / `C-w` / `C-y` | コピー / カット / ペースト |
| `C-g` | キャンセル |

### 検索と置換

| キー | 動作 |
|------|------|
| `C-s` / `C-r` | 前方検索 / 後方検索 |
| `M-%` | 対話的置換 |

対話的置換: `y` (置換), `n` (スキップ), `!` (残り全置換), `q` (終了)

検索/置換はテキストとHEXパターンの両方に対応:
- `hello` — ASCII文字列
- `48 65 6C 6C 6F` — スペース区切りHEX
- `48656C6C6F` — 連続HEX

### ファイル操作

| キー | 動作 |
|------|------|
| `C-x C-s` | 保存 |
| `C-x C-w` | 別名保存 |
| `C-x C-f` | ファイルを開く |
| `C-x k` | バッファを閉じる（空のバッファに） |
| `C-x C-c` | 終了 |

未保存時の確認: `y` (保存して続行), `n` (破棄), `c` (キャンセル)

### ナビゲーション

| キー | 動作 |
|------|------|
| `M-g` | アドレスジャンプ（16進: `0x100`, `100h`、10進も可） |

### コマンド (M-x)

| コマンド | 動作 |
|----------|------|
| `fill` / `f` | 選択範囲を指定バイトで埋める（例: `00`, `FF`） |
| `insert` / `i` | カーソル位置にNバイト挿入（例: `16 00`, `0x10 FF`） |
| `goto` / `g` | アドレスジャンプ |
| `save` / `s` | 保存 |
| `quit` / `q` | 終了 |
| `help` / `?` | コマンド一覧 |

### 表示

| キー | 動作 |
|------|------|
| `F2` | エンコーディング切替 |

---

## 入力モード

### HEXモード（デフォルト）

16進数（`0-9`, `A-F`）を入力してバイトを直接編集。
全角文字（`０-９`, `Ａ-Ｆ`）も自動変換。

### ASCIIモード

`Tab` で切り替え。任意の文字を入力可能。現在のエンコーディングに従ってバイト列に変換:
- UTF-8: `あ` → `E3 81 82` (3バイト)
- Shift-JIS: `あ` → `82 A0` (2バイト)

---

## ペースト形式

ブラケットペーストは形式を自動判別:
- `48 65 6C 6C 6F` — スペース区切りHEX
- `48656C6C6F` — 連続HEX
- `Hello` — 生テキスト（バイト列として）

---

## クリップボード連携

- **OSC 52** エスケープシーケンスでシステムクリップボードにコピー
- iTerm2, kitty, alacritty, WezTerm でSSH越しでも動作
- **tmux**: `.tmux.conf` に `set -g allow-passthrough on` を追加

---

## インスピレーション

- [ze](https://github.com/sanohiro/ze) — ゼロレイテンシのEmacs風エディタ
- [hexyl](https://github.com/sharkdp/hexyl) — Hexビューア
- [Stirling](https://github.com/nickg/stirling) — GUIバイナリエディタ

---

## bx — パイプ用バイナリツール

hxに同梱のUnixスタイルバイナリ操作ツール。

```bash
# HEXパターン検索
echo -n "Hello" | bx find 6C6C        # "ll"を検索
bx find DEADBEEF -i firmware.bin

# バイト範囲抽出
bx slice 0x100:0x200 -i file.bin      # バイト抽出
bx slice 0:512 -i file.bin -x         # HEXダンプ

# パターン置換
bx replace FF00 AA55 < in.bin > out.bin
bx replace --all 00 FF < in > out     # 全置換

# オフセット指定パッチ
bx patch 0x100=DEAD 0x200=BEEF < in > out

# ファイル情報（サイズ、エントロピー）
bx info -i file.bin

# HEX ⇔ バイナリ変換
echo -n "Hello" | bx conv bin2hex     # 48 65 6C 6C 6F
echo "48656C6C6F" | bx conv hex2bin   # Hello
```

---

## ライセンス

MIT

---

*"テキストを編集するようにバイナリを編集。"*
