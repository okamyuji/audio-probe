fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // バージョン情報の埋め込み
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("cargo:rustc-env=GIT_HASH={}", git_hash());
}

fn git_hash() -> String {
    use std::process::Command;

    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
