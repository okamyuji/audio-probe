use anyhow::{Context, Result};
use clap::Parser;
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum AudioProbeError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("Invalid audio file: {path} - {reason}")]
    InvalidAudioFile { path: PathBuf, reason: String },
    #[error("FFprobe not found. Please install FFmpeg.")]
    FFprobeNotFound,
    #[error("FFprobe execution error: {0}")]
    FFprobeError(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Processing error: {0}")]
    Processing(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub file_path: PathBuf,
    pub file_size: u64,
    pub duration_seconds: f64,
    pub bit_rate: i64,
    pub sample_rate: i32,
    pub channels: i32,
    pub codec_name: String,
    pub codec_long_name: String,
    pub format_name: String,
    pub format_long_name: String,
    pub has_video: bool,
    pub metadata: HashMap<String, String>,
    pub processing_time_ms: u64,
}

// FFprobeのJSON出力構造
#[derive(Debug, Deserialize)]
struct FFProbeOutput {
    format: Option<FFProbeFormat>,
    streams: Vec<FFProbeStream>,
}

#[derive(Debug, Deserialize)]
struct FFProbeFormat {
    #[allow(dead_code)]
    filename: String,
    format_name: String,
    format_long_name: String,
    duration: Option<String>,
    #[allow(dead_code)]
    size: Option<String>,
    bit_rate: Option<String>,
    tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct FFProbeStream {
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    codec_type: String,
    sample_rate: Option<String>,
    channels: Option<i32>,
    bit_rate: Option<String>,
}

impl AudioInfo {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            file_size: 0,
            duration_seconds: 0.0,
            bit_rate: 0,
            sample_rate: 0,
            channels: 0,
            codec_name: String::new(),
            codec_long_name: String::new(),
            format_name: String::new(),
            format_long_name: String::new(),
            has_video: false,
            metadata: HashMap::new(),
            processing_time_ms: 0,
        }
    }
}

pub struct AudioProbe {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    use_ffprobe: bool,
}

impl AudioProbe {
    pub async fn new(max_concurrent: usize) -> Result<Self> {
        // ffprobeが利用可能かチェック
        let use_ffprobe = Self::check_ffprobe().await;

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
            use_ffprobe,
        })
    }

    async fn check_ffprobe() -> bool {
        Command::new("ffprobe")
            .arg("-version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub async fn analyze_file(&self, path: PathBuf) -> Result<AudioInfo, AudioProbeError> {
        let _permit = self.semaphore.acquire().await.unwrap();
        let start_time = Instant::now();

        debug!("Analyzing file: {:?}", path);

        if !path.exists() {
            return Err(AudioProbeError::FileNotFound { path });
        }

        let mut audio_info = AudioInfo::new(path.clone());

        // ファイルサイズ取得
        if let Ok(metadata) = std::fs::metadata(&path) {
            audio_info.file_size = metadata.len();
        }

        if self.use_ffprobe {
            // FFprobeを使用して実際の解析
            match self.analyze_with_ffprobe(&path).await {
                Ok(info) => {
                    audio_info = info;
                }
                Err(e) => {
                    warn!("FFprobe analysis failed for {:?}: {}", path, e);
                    // フォールバック：基本的な推定
                    self.fallback_analysis(&mut audio_info, &path);
                }
            }
        } else {
            // FFprobeが利用できない場合の推定
            self.fallback_analysis(&mut audio_info, &path);
        }

        // デフォルトメタデータの設定
        if audio_info.metadata.get("title").is_none() {
            if let Some(file_stem) = path.file_stem() {
                if let Some(name) = file_stem.to_str() {
                    audio_info
                        .metadata
                        .insert("title".to_string(), name.to_string());
                }
            }
        }
        if audio_info.metadata.get("artist").is_none() {
            audio_info
                .metadata
                .insert("artist".to_string(), "Unknown Artist".to_string());
        }
        if audio_info.metadata.get("album").is_none() {
            audio_info
                .metadata
                .insert("album".to_string(), "Unknown Album".to_string());
        }

        audio_info.processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(audio_info)
    }

    async fn analyze_with_ffprobe(&self, path: &Path) -> Result<AudioInfo, AudioProbeError> {
        let output = Command::new("ffprobe")
            .args(&[
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .await
            .map_err(|e| {
                AudioProbeError::FFprobeError(format!("Failed to execute ffprobe: {}", e))
            })?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(AudioProbeError::FFprobeError(format!(
                "FFprobe failed: {}",
                error_msg
            )));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let probe_data: FFProbeOutput = serde_json::from_str(&json_str).map_err(|e| {
            AudioProbeError::Processing(format!("Failed to parse ffprobe output: {}", e))
        })?;

        let mut audio_info = AudioInfo::new(path.to_path_buf());

        // ファイルサイズ取得
        if let Ok(metadata) = std::fs::metadata(path) {
            audio_info.file_size = metadata.len();
        }

        // フォーマット情報
        if let Some(format) = probe_data.format {
            audio_info.format_name = format.format_name;
            audio_info.format_long_name = format.format_long_name;

            if let Some(duration_str) = format.duration {
                audio_info.duration_seconds = duration_str.parse::<f64>().unwrap_or(0.0);
            }

            if let Some(bit_rate_str) = format.bit_rate {
                audio_info.bit_rate = bit_rate_str.parse::<i64>().unwrap_or(0);
            }

            // メタデータ
            if let Some(tags) = format.tags {
                for (key, value) in tags {
                    audio_info.metadata.insert(key.to_lowercase(), value);
                }
            }
        }

        // ストリーム情報
        let mut audio_stream = None;
        for stream in probe_data.streams {
            if stream.codec_type == "audio" && audio_stream.is_none() {
                audio_stream = Some(stream);
            } else if stream.codec_type == "video" {
                audio_info.has_video = true;
            }
        }

        if let Some(stream) = audio_stream {
            if let Some(codec_name) = stream.codec_name {
                audio_info.codec_name = codec_name;
            }
            if let Some(codec_long_name) = stream.codec_long_name {
                audio_info.codec_long_name = codec_long_name;
            }
            if let Some(sample_rate_str) = stream.sample_rate {
                audio_info.sample_rate = sample_rate_str.parse::<i32>().unwrap_or(0);
            }
            if let Some(channels) = stream.channels {
                audio_info.channels = channels;
            }

            // ストリームのビットレートがある場合、フォーマットのビットレートよりも優先
            if let Some(bit_rate_str) = stream.bit_rate {
                if let Ok(stream_bit_rate) = bit_rate_str.parse::<i64>() {
                    if stream_bit_rate > 0 && audio_info.bit_rate == 0 {
                        audio_info.bit_rate = stream_bit_rate;
                    }
                }
            }
        }

        Ok(audio_info)
    }

    fn fallback_analysis(&self, audio_info: &mut AudioInfo, path: &Path) {
        // 基本的な情報を設定（実際のFFmpeg解析の代わり）
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                audio_info.format_name = ext_str.to_lowercase();
                audio_info.codec_name = ext_str.to_lowercase();

                // 拡張子に基づく基本情報の推定
                match ext_str.to_lowercase().as_str() {
                    "mp3" => {
                        audio_info.codec_long_name = "MP3 (MPEG audio layer 3)".to_string();
                        audio_info.format_long_name = "MP2/3 (MPEG audio layer 2/3)".to_string();
                        audio_info.sample_rate = 44100;
                        audio_info.channels = 2;
                        audio_info.bit_rate = 320000;
                    }
                    "wav" => {
                        audio_info.codec_name = "pcm_s16le".to_string();
                        audio_info.codec_long_name = "PCM signed 16-bit little-endian".to_string();
                        audio_info.format_long_name = "WAV / WAVE (Waveform Audio)".to_string();
                        audio_info.sample_rate = 44100;
                        audio_info.channels = 2;
                        audio_info.bit_rate = 44100 * 2 * 16; // 1411200
                    }
                    "flac" => {
                        audio_info.codec_long_name = "FLAC (Free Lossless Audio Codec)".to_string();
                        audio_info.format_long_name = "raw FLAC".to_string();
                        audio_info.sample_rate = 44100;
                        audio_info.channels = 2;
                    }
                    _ => {
                        audio_info.codec_long_name = format!("{} audio", ext_str.to_uppercase());
                        audio_info.format_long_name = format!("{} format", ext_str.to_uppercase());
                        audio_info.sample_rate = 44100;
                        audio_info.channels = 2;
                        audio_info.bit_rate = 320000;
                    }
                }
            }
        }

        // ファイルサイズに基づく継続時間の推定
        if audio_info.bit_rate > 0 {
            audio_info.duration_seconds =
                (audio_info.file_size * 8) as f64 / audio_info.bit_rate as f64;
        } else {
            // デフォルトの継続時間（5分）
            audio_info.duration_seconds = 300.0;
        }
    }

    pub async fn process_files(
        &self,
        paths: Vec<PathBuf>,
    ) -> Vec<Result<AudioInfo, AudioProbeError>> {
        let total_files = paths.len();
        info!(
            "Processing {} files with max {} concurrent operations",
            total_files, self.max_concurrent
        );

        let multi_progress = MultiProgress::new();
        let progress_bar = multi_progress.add(ProgressBar::new(total_files as u64));
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        let results = stream::iter(paths)
            .map(|path| {
                let probe = self.clone();
                let pb = progress_bar.clone();
                async move {
                    let result = probe.analyze_file(path).await;
                    pb.inc(1);
                    result
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        progress_bar.finish_with_message("Complete!");

        results
    }

    pub fn collect_audio_files<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<PathBuf>> {
        let audio_extensions = [
            "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", "mp2", "ac3", "dts", "ape",
            "aiff", "au", "ra", "amr", "webm", "mkv", "m4b", "m4p",
        ];

        let mut audio_files = Vec::new();

        for entry in WalkDir::new(root_path).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if audio_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            audio_files.push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(audio_files)
    }
}

impl Clone for AudioProbe {
    fn clone(&self) -> Self {
        Self {
            semaphore: Arc::clone(&self.semaphore),
            max_concurrent: self.max_concurrent,
            use_ffprobe: self.use_ffprobe,
        }
    }
}

#[derive(Parser)]
#[command(author, version = "0.2.0", about, long_about = None)]
struct Args {
    /// 解析する音声ファイルまたはディレクトリのパス
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// 最大同時処理数（デフォルト: 50）
    #[arg(short = 'j', long, default_value = "50")]
    max_concurrent: usize,

    /// JSON形式で出力
    #[arg(long)]
    json: bool,

    /// 詳細出力
    #[arg(short, long)]
    verbose: bool,

    /// エラーのみ表示
    #[arg(short, long)]
    quiet: bool,

    /// 再帰的にサブディレクトリを処理
    #[arg(short, long)]
    recursive: bool,

    /// 出力ファイル（指定しない場合は標準出力）
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // ログ設定
    let log_level = if args.quiet {
        "error"
    } else if args.verbose {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(format!("audio_probe={}", log_level))
        .init();

    println!("🎵 Audio Probe - 高性能音声ファイル解析ツール v0.2.0");

    if args.paths.is_empty() {
        eprintln!("エラー: 少なくとも1つのファイルまたはディレクトリパスを指定してください");
        std::process::exit(1);
    }

    let probe = AudioProbe::new(args.max_concurrent)
        .await
        .context("Failed to initialize AudioProbe")?;

    if probe.use_ffprobe {
        println!("FFprobeを使用して実際の音声ファイル情報を解析します");
    } else {
        println!("警告: FFprobeが見つかりません。基本的な情報推定を行います");
        println!("FFmpegをインストールすることで、より正確な解析が可能になります");
    }

    let mut all_files = Vec::new();

    // パス処理
    for path in &args.paths {
        if path.is_file() {
            all_files.push(path.clone());
        } else if path.is_dir() {
            if args.recursive {
                let collected = probe
                    .collect_audio_files(path)
                    .with_context(|| format!("Failed to collect files from {:?}", path))?;
                all_files.extend(collected);
            } else {
                // 非再帰的な場合、ディレクトリ内の音声ファイルのみ
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                            let file_path = entry.path();
                            if let Some(extension) = file_path.extension() {
                                if let Some(ext_str) = extension.to_str() {
                                    let audio_extensions = [
                                        "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus",
                                        "mp2", "ac3", "dts", "ape", "aiff", "au", "ra", "amr",
                                        "webm", "mkv", "m4b", "m4p",
                                    ];
                                    if audio_extensions.contains(&ext_str.to_lowercase().as_str()) {
                                        all_files.push(file_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            warn!("Path does not exist: {:?}", path);
        }
    }

    if all_files.is_empty() {
        eprintln!("警告: 処理する音声ファイルが見つかりませんでした");
        return Ok(());
    }

    info!("Found {} audio files to process", all_files.len());

    let start_time = Instant::now();
    let results = probe.process_files(all_files).await;
    let total_time = start_time.elapsed();

    // 結果の処理と出力
    let mut successful = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(audio_info) => successful.push(audio_info),
            Err(error) => errors.push(error),
        }
    }

    // 統計情報
    info!("Processing completed in {:.2}s", total_time.as_secs_f64());
    info!("Successfully processed: {}", successful.len());
    if !errors.is_empty() {
        warn!("Failed to process: {}", errors.len());
    }

    // 統計情報の計算
    let total_duration: f64 = successful.iter().map(|info| info.duration_seconds).sum();
    let total_size: u64 = successful.iter().map(|info| info.file_size).sum();

    // 出力
    let output_content = if args.json {
        // JSON出力
        let output_data = serde_json::json!({
            "summary": {
                "total_files": successful.len() + errors.len(),
                "successful": successful.len(),
                "failed": errors.len(),
                "processing_time_seconds": total_time.as_secs_f64(),
                "total_duration_seconds": total_duration,
                "total_size_bytes": total_size,
            },
            "successful_files": successful,
            "errors": errors.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        });
        serde_json::to_string_pretty(&output_data)?
    } else {
        // 標準出力フォーマット
        let mut output = String::new();

        output.push_str(&format!("=== 音声ファイル分析結果 ===\n"));
        output.push_str(&format!("処理時間: {:.2}秒\n", total_time.as_secs_f64()));
        output.push_str(&format!(
            "成功: {}, 失敗: {}\n",
            successful.len(),
            errors.len()
        ));
        output.push_str(&format!(
            "総継続時間: {}\n",
            format_duration(total_duration)
        ));
        output.push_str(&format!("総サイズ: {}\n\n", format_bytes(total_size)));

        for audio_info in &successful {
            output.push_str(&format!("📁 ファイル: {:?}\n", audio_info.file_path));
            output.push_str(&format!(
                "   サイズ: {}\n",
                format_bytes(audio_info.file_size)
            ));
            output.push_str(&format!(
                "   継続時間: {}\n",
                format_duration(audio_info.duration_seconds)
            ));
            output.push_str(&format!(
                "   ビットレート: {}\n",
                format_bitrate(audio_info.bit_rate)
            ));
            output.push_str(&format!(
                "   サンプルレート: {} Hz\n",
                audio_info.sample_rate
            ));
            output.push_str(&format!("   チャンネル数: {}\n", audio_info.channels));
            output.push_str(&format!(
                "   コーデック: {} ({})\n",
                audio_info.codec_name, audio_info.codec_long_name
            ));
            output.push_str(&format!(
                "   フォーマット: {} ({})\n",
                audio_info.format_name, audio_info.format_long_name
            ));
            output.push_str(&format!(
                "   動画含む: {}\n",
                if audio_info.has_video {
                    "はい"
                } else {
                    "いいえ"
                }
            ));
            output.push_str(&format!(
                "   処理時間: {}ms\n",
                audio_info.processing_time_ms
            ));

            if !audio_info.metadata.is_empty() {
                output.push_str("   メタデータ:\n");
                for (key, value) in &audio_info.metadata {
                    if !value.is_empty() {
                        output.push_str(&format!("     {}: {}\n", key, value));
                    }
                }
            }
            output.push_str("\n");
        }

        if !errors.is_empty() {
            output.push_str("=== エラー ===\n");
            for error in &errors {
                output.push_str(&format!("❌ {}\n", error));
            }
        }

        output
    };

    // 出力先の決定
    if let Some(output_path) = args.output {
        std::fs::write(output_path, output_content)?;
    } else {
        print!("{}", output_content);
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn format_duration(seconds: f64) -> String {
    let hours = (seconds as u64) / 3600;
    let minutes = ((seconds as u64) % 3600) / 60;
    let secs = (seconds as u64) % 60;

    if hours > 0 {
        format!("{}時間{}分{}秒", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}分{}秒", minutes, secs)
    } else {
        format!("{:.1}秒", seconds)
    }
}

fn format_bitrate(bitrate: i64) -> String {
    if bitrate >= 1_000_000 {
        format!("{:.1} Mbps", bitrate as f64 / 1_000_000.0)
    } else if bitrate >= 1_000 {
        format!("{} kbps", bitrate / 1_000)
    } else if bitrate > 0 {
        format!("{} bps", bitrate)
    } else {
        "N/A".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_probe_creation() {
        let probe = AudioProbe::new(10).await;
        assert!(probe.is_ok());
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let probe = AudioProbe::new(1).await.unwrap();
        let result = probe.analyze_file(PathBuf::from("nonexistent.mp3")).await;
        assert!(matches!(result, Err(AudioProbeError::FileNotFound { .. })));
    }

    #[test]
    fn test_audio_info_creation() {
        let path = PathBuf::from("test.mp3");
        let info = AudioInfo::new(path.clone());
        assert_eq!(info.file_path, path);
        assert_eq!(info.duration_seconds, 0.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 bytes");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30.0), "30.0秒");
        assert_eq!(format_duration(90.0), "1分30秒");
        assert_eq!(format_duration(3661.0), "1時間1分1秒");
    }

    #[test]
    fn test_format_bitrate() {
        assert_eq!(format_bitrate(128), "128 bps");
        assert_eq!(format_bitrate(128000), "128 kbps");
        assert_eq!(format_bitrate(1000000), "1.0 Mbps");
        assert_eq!(format_bitrate(0), "N/A");
    }
}
