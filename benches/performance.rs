use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;
use std::time::Duration;

// 注意: このベンチマークはAudioProbeの実装をテストするためのものです
// 実際の使用には音声ファイルが必要です

fn benchmark_file_collection(c: &mut Criterion) {
    use walkdir::WalkDir;
    
    c.bench_function("collect_audio_files", |b| {
        b.iter(|| {
            let audio_extensions = [
                "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", 
                "mp2", "ac3", "dts", "ape", "aiff", "au", "ra", "amr"
            ];

            let mut audio_files = Vec::new();
            let test_dir = black_box("./");

            for entry in WalkDir::new(test_dir).follow_links(false) {
                if let Ok(entry) = entry {
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
            }
            
            black_box(audio_files)
        })
    });
}

fn benchmark_path_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_processing");
    
    for file_count in [10, 100, 1000].iter() {
        let paths: Vec<PathBuf> = (0..*file_count)
            .map(|i| PathBuf::from(format!("test_audio_{}.mp3", i)))
            .collect();
        
        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("path_validation", file_count),
            file_count,
            |b, &_file_count| {
                b.iter(|| {
                    let results: Vec<bool> = paths.iter()
                        .map(|path| path.exists())
                        .collect();
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_metadata_parsing(c: &mut Criterion) {
    use std::collections::HashMap;
    
    c.bench_function("metadata_creation", |b| {
        b.iter(|| {
            let mut metadata: HashMap<String, String> = HashMap::new();
            
            // 典型的なメタデータを模擬
            let sample_metadata = vec![
                ("title", "Sample Track"),
                ("artist", "Sample Artist"),
                ("album", "Sample Album"),
                ("date", "2023"),
                ("genre", "Electronic"),
                ("track", "1/12"),
                ("albumartist", "Sample Artist"),
                ("composer", "Sample Composer"),
            ];
            
            for (key, value) in sample_metadata {
                metadata.insert(key.to_string(), value.to_string());
            }
            
            black_box(metadata)
        })
    });
}

fn benchmark_concurrent_simulation(c: &mut Criterion) {
    use tokio::runtime::Runtime;
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_simulation");
    
    for concurrency in [1, 5, 10, 25, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("semaphore_control", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let semaphore = Arc::new(Semaphore::new(concurrency));
                    let tasks: Vec<_> = (0..100).map(|_| {
                        let sem = Arc::clone(&semaphore);
                        tokio::spawn(async move {
                            let _permit = sem.acquire().await.unwrap();
                            // ファイル処理をシミュレート
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            42
                        })
                    }).collect();
                    
                    let results: Vec<_> = futures::future::join_all(tasks).await;
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_file_collection,
    benchmark_path_processing,
    benchmark_metadata_parsing,
    benchmark_concurrent_simulation
);
criterion_main!(benches);
