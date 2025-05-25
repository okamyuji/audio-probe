#!/bin/bash

set -e

echo "🎵 Audio Probe セットアップスクリプト"
echo "================================="

# OS検出
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    OS="windows"
else
    echo "❌ サポートされていないOS: $OSTYPE"
    exit 1
fi

echo "🖥️  検出されたOS: $OS"

# FFmpegのインストール
echo "📦 FFmpeg依存関係をインストール中..."

case $OS in
    "linux")
        if command -v apt-get &> /dev/null; then
            sudo apt update
            sudo apt install -y ffmpeg libavformat-dev libavcodec-dev libavutil-dev \
                               libavfilter-dev libavdevice-dev libswscale-dev \
                               libswresample-dev pkg-config
        elif command -v yum &> /dev/null; then
            sudo yum install -y epel-release
            sudo yum install -y ffmpeg-devel pkg-config
        elif command -v dnf &> /dev/null; then
            sudo dnf install -y ffmpeg-devel pkg-config
        else
            echo "❌ サポートされていないLinuxディストリビューション"
            exit 1
        fi
        ;;
    "macos")
        if ! command -v brew &> /dev/null; then
            echo "❌ Homebrewが必要です。https://brew.sh からインストールしてください。"
            exit 1
        fi
        brew install ffmpeg pkg-config
        ;;
    "windows")
        echo "⚠️  Windowsでは手動でFFmpegをインストールしてください："
        echo "   1. https://www.gyan.dev/ffmpeg/builds/ からFFmpegをダウンロード"
        echo "   2. PATHに追加"
        echo "   3. または vcpkg install ffmpeg:x64-windows を使用"
        ;;
esac

# Rustの確認
echo "🦀 Rustツールチェーンを確認中..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Rustが見つかりません。https://rustup.rs からインストールしてください。"
    exit 1
fi

echo "✅ Rust $(rustc --version)"

# プロジェクトのビルド
echo "🔨 プロジェクトをビルド中..."
cargo build --release

# テストの実行
echo "🧪 テストを実行中..."
cargo test

# 成功メッセージ
echo ""
echo "🎉 セットアップ完了！"
echo ""
echo "使用方法:"
echo "  cargo run -- --help              # ヘルプを表示"
echo "  cargo run -- sample.mp3          # 単一ファイル解析"
echo "  cargo run -- -r /path/to/music   # ディレクトリ再帰解析"
echo ""
echo "実行可能ファイル: ./target/release/audio-probe"
