// Commands live in Kotlin; Tauri forwards `plugin:encedo-push|<command>` to the
// native plugin when no Rust handler claims it. The list here feeds the ACL.
const COMMANDS: &[&str] = &[
    "get_token",
    "request_permissions",
    "check_permissions",
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
