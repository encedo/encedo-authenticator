const COMMANDS: &[&str] = &["wrap", "unwrap", "device_name", "protection", "bound_to_user"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").ios_path("ios").build();
}
