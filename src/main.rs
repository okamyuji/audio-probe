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
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum AudioProbeError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("Invalid audio file: {path} - {reason}")]
    InvalidAudioFile { path: PathBuf, reason: String },
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
}

impl AudioProbe {
    pub fn new(max_concurrent: usize) -> Result<Self> {
        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        })
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
                        audio_info.codec_long_name = "PCM signed 16-bit little-endian".to_string();
                        audio_info.format_long_name = "WAV / WAVE (Waveform Audio)".to_string();
                        audio_info.sample_rate = 44100;
                        audio_info.channels = 2;
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
                    }
                }
            }
        }

        // ファイルサイズに基づく継続時間の推定
        if audio_info.bit_rate > 0 {
            audio_info.duration_seconds = (audio_info.file_size * 8) as f64 / audio_info.bit_rate as f64;
        } else {
            // デフォルトの継続時間（5分）
            audio_info.duration_seconds = 300.0;
        }

        audio_info.processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(audio_info)
    }

    pub async fn process_files(&self, paths: Vec<PathBuf>) -> Vec<Result<AudioInfo, AudioProbeError>> {
        let total_files = paths.len();
        info!("Processing {} files with max {} concurrent operations", total_files, self.max_concurrent);

        let multi_progress = MultiProgress::new();
        let progress_bar = multi_progress.add(ProgressBar::new(total_files as u64));
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        let progress_handle = tokio::spawn(async move {
            multi_progress.join().unwrap();
        });

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
        let _ = progress_handle.await;

        results
    }

    pub fn collect_audio_files<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<PathBuf>> {
        let audio_extensions = [
            "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", 
            "mp2", "ac3", "dts", "ape", "aiff", "au", "ra", "amr"
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
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
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

    println!("🎵 Audio Probe - 高性能音声ファイル解析ツール (デモ版)");
    println!("注意: この版では実際のFFmpeg解析の代わりに基本的な情報推定を行います");

    if args.paths.is_empty() {
        eprintln!("エラー: 少なくとも1つのファイルまたはディレクトリパスを指定してください");
        std::process::exit(1);
    }

    let probe = AudioProbe::new(args.max_concurrent)
        .context("Failed to initialize AudioProbe")?;

    let mut all_files = Vec::new();

    // パス処理
    for path in &args.paths {
        if path.is_file() {
            all_files.push(path.clone());
        } else if path.is_dir() {
            if args.recursive {
                let collected = probe.collect_audio_files(path)
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
                                        "mp2", "ac3", "dts", "ape", "aiff", "au", "ra", "amr"
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

    // 出力
    let output_content = if args.json {
        // JSON出力
        let output_data = serde_json::json!({
            "summary": {
                "total_files": successful.len() + errors.len(),
                "successful": successful.len(),
                "failed": errors.len(),
                "processing_time_seconds": total_time.as_secs_f64()
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
        output.push_str(&format!("成功: {}, 失敗: {}\n\n", successful.len(), errors.len()));

        for audio_info in &successful {
            output.push_str(&format!("📁 ファイル: {:?}\n", audio_info.file_path));
            output.push_str(&format!("   サイズ: {} bytes\n", audio_info.file_size));
            output.push_str(&format!("   継続時間: {:.2}秒\n", audio_info.duration_seconds));
            output.push_str(&format!("   ビットレート: {} bps\n", audio_info.bit_rate));
            output.push_str(&format!("   サンプルレート: {} Hz\n", audio_info.sample_rate));
            output.push_str(&format!("   チャンネル数: {}\n", audio_info.channels));
            output.push_str(&format!("   コーデック: {} ({})\n", audio_info.codec_name, audio_info.codec_long_name));
            output.push_str(&format!("   フォーマット: {} ({})\n", audio_info.format_name, audio_info.format_long_name));
            output.push_str(&format!("   動画含む: {}\n", if audio_info.has_video { "はい" } else { "いいえ" }));
            output.push_str(&format!("   処理時間: {}ms\n", audio_info.processing_time_ms));
            
            if !audio_info.metadata.is_empty() {
                output.push_str("   メタデータ:\n");
                for (key, value) in &audio_info.metadata {
                    output.push_str(&format!("     {}: {}\n", key, value));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_probe_creation() {
        let probe = AudioProbe::new(10);
        assert!(probe.is_ok());
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let probe = AudioProbe::new(1).unwrap();
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
}
