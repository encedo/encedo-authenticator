const COMMANDS: &[&str] = &["wrap", "unwrap", "device_name"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").ios_path("ios").build();
}
