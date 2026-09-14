// Commands live in Kotlin; Tauri forwards `plugin:encedo-update|<command>` to the
// native plugin. Android only: Apple has no equivalent, so there is no iOS half.
const COMMANDS: &[&str] = &["check", "start", "open_store"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
