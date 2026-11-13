fn main() {
    // DEV-512: the login context names the build, `<version>+<commit>`. CI has
    // GITHUB_SHA; a local build asks git; neither gives "unknown".
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .filter(|s| s.len() >= 7)
        .map(|s| s[..7].to_string())
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--short=7", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=APP_COMMIT={commit}");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    tauri_build::build()
}
