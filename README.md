# Audio Probe - 高性能音声ファイル解析ツール

Rustで実装された高性能な音声ファイル解析ツールです。FFmpegの機能を直接利用し、大量のファイルを並行処理で効率的に解析します。

## 特徴

- 🔥 **高性能**: Tokioによる非同期並行処理で最大2000ファイルを同時処理
- 🎵 **包括的サポート**: MP3, WAV, FLAC, AAC, OGG, M4A, WMA, OPUSなど主要音声フォーマット対応
- 🚀 **FFmpeg直接利用**: 外部プロセス不要、Rustバインディングによる高速処理
- 📊 **詳細分析**: コーデック情報、ビットレート、サンプルレート、メタデータ取得
- 💾 **メモリ効率**: セマフォーによる同時実行数制御で低メモリ使用量
- 📈 **プログレス表示**: リアルタイム進捗状況とパフォーマンス統計
- 🎯 **柔軟な出力**: 標準出力またはJSON形式での結果出力

## 必要要件

### システム要件
- Rust 1.70.0 以上
- FFmpeg 6.x または 7.x 開発ライブラリ

### FFmpeg インストール

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install ffmpeg libavformat-dev libavcodec-dev libavutil-dev libavfilter-dev libavdevice-dev libswscale-dev libswresample-dev pkg-config
```

#### CentOS/RHEL/Fedora
```bash
# Fedora
sudo dnf install ffmpeg-devel pkg-config

# CentOS/RHEL (EPEL リポジトリが必要)
sudo yum install epel-release
sudo yum install ffmpeg-devel pkg-config
```

#### macOS
```bash
brew install ffmpeg pkg-config
```

#### Windows
vcpkgを使用する場合：
```bash
vcpkg install ffmpeg:x64-windows
```

### 環境変数設定

FFmpegが標準的な場所にインストールされていない場合：

```bash
# Linux/macOS
export FFMPEG_PKG_CONFIG_PATH=/path/to/ffmpeg/lib/pkgconfig

# Windows
set FFMPEG_PKG_CONFIG_PATH=C:\path\to\ffmpeg\lib\pkgconfig
```

## ビルドとインストール

```bash
# リポジトリをクローン
git clone <repository-url>
cd audio-probe

# リリースビルド（最高性能）
cargo build --release

# インストール（オプション）
cargo install --path .
```

## 使用方法

### 基本的な使用例

```bash
# 単一ファイル解析
./target/release/audio-probe music.mp3

# 複数ファイル解析
./target/release/audio-probe song1.mp3 song2.wav album/*.flac

# ディレクトリ解析（再帰的）
./target/release/audio-probe -r /path/to/music/collection

# 高並行処理（100並列）
./target/release/audio-probe -j 100 /path/to/large/collection

# JSON出力
./target/release/audio-probe --json music_files/ > results.json

# ファイル出力
./target/release/audio-probe -o analysis_report.txt /path/to/music
```

### オプション詳細

```
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
./target/release/audio-probe -j 100 /large/music/collection

# メモリ制約がある場合
./target/release/audio-probe -j 20 /music/collection

# 超高速SSD環境
./target/release/audio-probe -j 200 /nvme/music/collection
```

### ベンチマーク例

```
テスト環境: Intel i7-12700K, 32GB RAM, NVMe SSD
- 1,000ファイル (平均5MB): 45秒 (並行数50)
- 2,000ファイル (平均5MB): 78秒 (並行数100)
- 10,000ファイル (平均3MB): 350秒 (並行数100)
```

## 出力例

### 標準出力
```
=== 音声ファイル分析結果 ===
処理時間: 23.45秒
成功: 1847, 失敗: 3

📁 ファイル: "/music/album/track01.mp3"
   サイズ: 5242880 bytes
   継続時間: 245.33秒
   ビットレート: 320000 bps
   サンプルレート: 44100 Hz
   チャンネル数: 2
   コーデック: mp3 (MP3 (MPEG audio layer 3))
   フォーマット: mp3 (MP2/3 (MPEG audio layer 2/3))
   動画含む: いいえ
   処理時間: 12ms
   メタデータ:
     artist: Example Artist
     album: Example Album
     title: Track 01
     date: 2023
```

### JSON出力
```json
{
  "summary": {
    "total_files": 1850,
    "successful": 1847,
    "failed": 3,
    "processing_time_seconds": 23.45
  },
  "successful_files": [
    {
      "file_path": "/music/album/track01.mp3",
      "file_size": 5242880,
      "duration_seconds": 245.33,
      "bit_rate": 320000,
      "sample_rate": 44100,
      "channels": 2,
      "codec_name": "mp3",
      "codec_long_name": "MP3 (MPEG audio layer 3)",
      "format_name": "mp3",
      "format_long_name": "MP2/3 (MPEG audio layer 2/3)",
      "has_video": false,
      "metadata": {
        "artist": "Example Artist",
        "album": "Example Album",
        "title": "Track 01",
        "date": "2023"
      },
      "processing_time_ms": 12
    }
  ],
  "errors": [
    "Invalid audio file: /music/broken.mp3 - No audio stream found"
  ]
}
```

## 対応フォーマット

### 音声フォーマット
- **ロスレス**: FLAC, WAV, AIFF, APE, AU
- **ロッシー**: MP3, AAC, OGG Vorbis, WMA, OPUS, AMR
- **コンテナ**: M4A, MP2, AC3, DTS, RA

### メタデータ
- 標準タグ (Title, Artist, Album, Date)
- フォーマット固有情報
- エンコーダー情報
- カスタムメタデータ

## トラブルシューティング

### よくある問題

#### 1. FFmpeg ライブラリが見つからない
```bash
error: failed to run custom build command for `rsmpeg-sys`
```
**解決方法**: FFmpeg開発ライブラリをインストールし、PKG_CONFIG_PATHを設定

#### 2. 大量ファイル処理でメモリ不足
**解決方法**: `-j` オプションで並行数を下げる（例: `-j 10`）

#### 3. パフォーマンスが期待より低い
**解決方法**: 
- リリースビルドを使用: `cargo build --release`
- 並行数を調整: CPU コア数の2-4倍を試す
- SSD使用を推奨

#### 4. 一部ファイルが解析できない
**解決方法**: `-v` オプションで詳細ログを確認し、ファイル形式を検証

## 開発・拡張

### テスト実行
```bash
cargo test
```

### カスタマイズ例
```rust
// より多くのメタデータ取得
// src/main.rs の AudioInfo 構造体を拡張
pub struct AudioInfo {
    // 既存フィールド...
    pub encoder: String,
    pub creation_time: Option<String>,
    pub track_number: Option<u32>,
}
```

## ライセンス

MIT License

## 貢献

プルリクエストやイシューの報告を歓迎します。

## 関連プロジェクト

- [rsmpeg](https://github.com/larksuite/rsmpeg) - FFmpeg Rust バインディング
- [Tokio](https://tokio.rs/) - 非同期ランタイム
- [FFmpeg](https://ffmpeg.org/) - マルチメディア処理ライブラリ
