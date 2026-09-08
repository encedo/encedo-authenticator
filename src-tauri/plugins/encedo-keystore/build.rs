const COMMANDS: &[&str] = &["wrap", "unwrap"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
