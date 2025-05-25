// examples/basic_usage.rs
// 基本的な使用例とAPIの説明

/// Audio Probe の基本的な使用例とサンプルコード
/// 
/// このファイルは使用例を示すためのドキュメンテーションです。
/// 実際の使用には以下のCLIコマンドを使用してください。

fn main() {
    println!("🎵 Audio Probe - 高性能音声ファイル解析ツール");
    println!("==============================================\n");
    
    println!("📖 基本的な使用方法:");
    print_basic_usage();
    
    println!("\n🔧 高度な使用例:");
    print_advanced_usage();
    
    println!("\n⚡ パフォーマンス最適化:");
    print_performance_tips();
    
    println!("\n🐛 トラブルシューティング:");
    print_troubleshooting();
}

fn print_basic_usage() {
    let examples = vec![
        ("単一ファイル解析", "cargo run -- music.mp3"),
        ("複数ファイル解析", "cargo run -- song1.mp3 song2.wav"),
        ("ディレクトリ解析", "cargo run -- /path/to/music/"),
        ("再帰的解析", "cargo run -- -r /path/to/music/collection/"),
        ("ヘルプ表示", "cargo run -- --help"),
    ];
    
    for (description, command) in examples {
        println!("  📝 {}: {}", description, command);
    }
}

fn print_advanced_usage() {
    let examples = vec![
        ("高並行処理 (100並列)", "cargo run -- -j 100 /large/music/collection/"),
        ("JSON形式で出力", "cargo run -- --json /music/ > results.json"),
        ("ファイルに結果出力", "cargo run -- -o report.txt /music/"),
        ("詳細ログ出力", "cargo run -- -v problematic_files/"),
        ("エラーのみ表示", "cargo run -- -q /music/collection/"),
        ("特定形式のみ処理", "find /music -name '*.flac' | xargs cargo run --"),
    ];
    
    for (description, command) in examples {
        println!("  🚀 {}: {}", description, command);
    }
}

fn print_performance_tips() {
    println!("  💡 リリースビルドを使用: cargo build --release");
    println!("  💡 並行数を調整: CPU コア数の2-4倍が最適");
    println!("  💡 SSDでの実行を推奨");
    println!("  💡 メモリ不足時は並行数を削減: -j 10");
    println!("  💡 大量ファイル処理: -j 50 から -j 200 の範囲で調整");
}

fn print_troubleshooting() {
    println!("  🔍 FFmpegエラー: pkg-config --libs libavformat で確認");
    println!("  🔍 ビルドエラー: Rustのバージョンを確認 (1.70.0以上)");
    println!("  🔍 メモリ不足: 並行数を削減 (-j オプション)");
    println!("  🔍 処理が遅い: リリースビルドを使用");
    println!("  🔍 ファイルが見つからない: パスを絶対パスで指定");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_main_function() {
        // main関数が正常に実行されることを確認
        main();
    }
}
