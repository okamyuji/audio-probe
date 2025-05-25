FROM rust:1.75-bullseye as builder

# FFmpeg依存関係のインストール
RUN apt-get update && apt-get install -y \
    ffmpeg \
    libavformat-dev \
    libavcodec-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    libswscale-dev \
    libswresample-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# 依存関係をコピーしてビルド（キャッシュ最適化）
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# ソースコードをコピーしてビルド
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# 実行用の軽量イメージ
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/audio-probe /usr/local/bin/audio-probe

# 非rootユーザーを作成
RUN useradd -r -s /bin/false audiouser
USER audiouser

WORKDIR /data

ENTRYPOINT ["audio-probe"]
CMD ["--help"]
