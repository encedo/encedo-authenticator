// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "tauri-plugin-encedo-keystore",
    platforms: [.iOS(.v13)],
    products: [
        .library(name: "tauri-plugin-encedo-keystore", type: .static, targets: ["EncedoKeystorePlugin"])
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "EncedoKeystorePlugin",
            dependencies: [.byName(name: "Tauri")],
            path: "Sources/EncedoKeystorePlugin")
    ]
)
