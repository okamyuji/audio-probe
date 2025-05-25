# Audio Probe - 高性能音声ファイル解析ツール

Rustで実装された高性能な音声ファイル解析ツールです。FFmpegの機能を直接利用し、大量のファイルを並行処理で効率的に解析します。

## プロジェクト状況

✅ **完了**: プロジェクト構造とコードベース  
✅ **完了**: Git リポジトリ初期化とコミット  
⚠️ **注意**: 現在はFFmpeg非依存のデモ版を提供中  

## 特徴

- 🔥 **高性能**: Tokioによる非同期並行処理で最大2000ファイルを同時処理
- 🎵 **包括的サポート**: MP3, WAV, FLAC, AAC, OGG, M4A, WMA, OPUSなど主要音声フォーマット対応
- 🚀 **FFmpeg直接利用**: 外部プロセス不要、Rustバインディングによる高速処理（フル版）
- 📊 **詳細分析**: コーデック情報、ビットレート、サンプルレート、メタデータ取得
- 💾 **メモリ効率**: セマフォーによる同時実行数制御で低メモリ使用量
- 📈 **プログレス表示**: リアルタイム進捗状況とパフォーマンス統計
- 🎯 **柔軟な出力**: 標準出力またはJSON形式での結果出力

## クイックスタート

### 1. 基本的なビルドとテスト

```bash
# プロジェクトディレクトリに移動
cd /Users/yujiokamoto/devs/rust/audio-probe

# ビルド（デバッグ版）
cargo build

# テストファイル作成
echo "Test audio content" > test.mp3
echo "Test WAV content" > test.wav

# プログラム実行（ヘルプ表示）
cargo run -- --help

# 基本テスト
cargo run -- test.mp3 test.wav

# 詳細出力でテスト
cargo run -- -v test.mp3

# JSON出力でテスト
cargo run -- --json test.mp3
```

### 2. リリースビルド（高性能）

```bash
# リリースビルド
cargo build --release

# リリース版で実行
./target/release/audio-probe --help
./target/release/audio-probe test.mp3
```

### 3. テスト実行

```bash
# ユニットテスト
cargo test

# ベンチマークテスト
cargo bench
```

## 必要要件

### システム要件

- Rust 1.70.0 以上
- FFmpeg 6.x または 7.x 開発ライブラリ（フル版用）

### 現在のデモ版

現在の実装はFFmpeg非依存のデモ版で、以下の機能を提供します

- ファイル拡張子ベースの基本情報推定
- 並行ファイル処理のデモンストレーション
- CLI インターフェースの完全な動作
- JSON/テキスト出力機能

### FFmpeg フル版への移行

FFmpeg統合版に移行するには、以下の手順を実行

1. **FFmpeg開発ライブラリのインストール**

   ```bash
   # macOS
   brew install ffmpeg pkg-config

   # Ubuntu/Debian
   sudo apt install ffmpeg libavformat-dev libavcodec-dev libavutil-dev pkg-config

   # Windows
   vcpkg install ffmpeg:x64-windows
   ```

2. **Cargo.tomlの更新**

   ```toml
   [dependencies]
   # 以下の行を追加
   rsmpeg = "0.15"
   ```

3. **main.rsのFFmpeg機能を有効化**

- `probe_with_ffmpeg`関数内の実際のFFmpeg処理コードを有効化
- デモ版の基本推定ロジックをFFmpeg解析に置き換え

## 使用方法

### 基本的な使用例

```bash
# 単一ファイル解析
cargo run -- music.mp3

# 複数ファイル解析
cargo run -- song1.mp3 song2.wav album/*.flac

# ディレクトリ解析（再帰的）
cargo run -- -r /path/to/music/collection

# 高並行処理（100並列）
cargo run -- -j 100 /path/to/large/collection

# JSON出力
cargo run -- --json music_files/ > results.json

# ファイル出力
cargo run -- -o analysis_report.txt /path/to/music
```

### オプション詳細

```text
audio-probe [オプション] <パス>...

引数:
    <PATH>...    解析する音声ファイルまたはディレクトリのパス

オプション:
    -j, --max-concurrent <数>  最大同時処理数 [デフォルト: 50]
        --json                 JSON形式で出力
    -v, --verbose              詳細出力
    -q, --quiet                エラーのみ表示
    -r, --recursive            再帰的にサブディレクトリを処理
    -o, --output <ファイル>    出力ファイル（指定しない場合は標準出力）
    -h, --help                 ヘルプメッセージを表示
    -V, --version              バージョン情報を表示
```

## パフォーマンス最適化

### 推奨設定

```bash
# 大量ファイル処理（メモリが潤沢な場合）
cargo run --release -- -j 100 /large/music/collection

# メモリ制約がある場合
cargo run --release -- -j 20 /music/collection

# 超高速SSD環境
cargo run --release -- -j 200 /nvme/music/collection
```

## 出力例

### 標準出力（デモ版）

```text
🎵 Audio Probe - 高性能音声ファイル解析ツール (デモ版)
注意: この版では実際のFFmpeg解析の代わりに基本的な情報推定を行います

=== 音声ファイル分析結果 ===
処理時間: 0.05秒
成功: 2, 失敗: 0

📁 ファイル: "test.mp3"
   サイズ: 25 bytes
   継続時間: 300.00秒
   ビットレート: 320000 bps
   サンプルレート: 44100 Hz
   チャンネル数: 2
   コーデック: mp3 (MP3 (MPEG audio layer 3))
   フォーマット: mp3 (MP2/3 (MPEG audio layer 2/3))
   動画含む: いいえ
   処理時間: 1ms
```

### JSON出力例

```json
{
  "summary": {
    "total_files": 2,
    "successful": 2,
    "failed": 0,
    "processing_time_seconds": 0.05
  },
  "successful_files": [
    {
      "file_path": "test.mp3",
      "file_size": 25,
      "duration_seconds": 300.0,
      "bit_rate": 320000,
      "sample_rate": 44100,
      "channels": 2,
      "codec_name": "mp3",
      "codec_long_name": "MP3 (MPEG audio layer 3)",
      "format_name": "mp3",
      "format_long_name": "MP2/3 (MPEG audio layer 2/3)",
      "has_video": false,
      "metadata": {},
      "processing_time_ms": 1
    }
  ],
  "errors": []
}
```

## 開発・拡張

### プロジェクト構造

```sh
audio-probe/
├── Cargo.toml              # プロジェクト設定と依存関係
├── build.rs                # ビルドスクリプト
├── README.md               # このファイル
├── .gitignore              # Git無視ファイル設定
├── src/
│   └── main.rs             # メインプログラム
├── examples/
│   └── basic_usage.rs      # 使用例とサンプルコード
├── benches/
│   └── performance.rs      # パフォーマンスベンチマーク
├── scripts/
│   └── setup.sh            # セットアップスクリプト
├── .github/
│   └── workflows/
│       └── ci.yml          # GitHub Actions CI/CD
└── Dockerfile              # コンテナ化設定
```

### テスト実行
```bash
# ユニットテスト
cargo test

# 詳細テスト出力
cargo test -- --nocapture

# 特定のテスト
cargo test test_audio_probe_creation
```

### ベンチマーク実行

```bash
cargo bench
```

### 例の実行

```bash
cargo run --example basic_usage
```

## Docker サポート

```bash
# イメージビルド
docker build -t audio-probe .

# コンテナ実行
docker run -v /path/to/music:/data audio-probe -r /data
```

## トラブルシューティング

### よくある問題

1. **ビルドエラー**:
   - Rustのバージョンを確認: `rustc --version` (1.70.0以上必要)
   - 依存関係を更新: `cargo update`

2. **FFmpegライブラリが見つからない（フル版）**:
   - FFmpeg開発ライブラリをインストール
   - `PKG_CONFIG_PATH`を適切に設定

3. **パフォーマンスが低い**:
   - リリースビルドを使用: `cargo build --release`
   - 並行数を調整: `-j` オプション

4. **メモリ不足**:
   - 並行数を削減: `-j 10` など

## 今後の開発計画

- [ ] FFmpeg統合の完成
- [ ] 追加音声フォーマットのサポート
- [ ] Web API版の開発
- [ ] GUI版の開発
- [ ] プラグインシステムの実装

## コントリビューション

プルリクエストやイシューの報告を歓迎します。詳細は[コントリビューションガイド](CONTRIBUTING.md)をご覧ください。

## ライセンス

MIT License

## 関連プロジェクト

- [rsmpeg](https://github.com/larksuite/rsmpeg) - FFmpeg Rust バインディング
- [Tokio](https://tokio.rs/) - 非同期ランタイム
- [FFmpeg](https://ffmpeg.org/) - マルチメディア処理ライブラリ
