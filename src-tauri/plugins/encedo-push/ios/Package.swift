// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "tauri-plugin-encedo-push",
    // macOS is named only because SPM resolves it for the package graph; Firebase asks for 10.15.
    platforms: [.iOS(.v14), .macOS(.v10_15)],
    products: [
        .library(name: "tauri-plugin-encedo-push", type: .static, targets: ["EncedoPushPlugin"])
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
        // The same transport as Android: the phone registers with FCM and the
        // backend addresses one kind of token. v1 did this through
        // cordova-plugin-firebasex; the Firebase project already has this
        // bundle id and an APNs key.
        .package(url: "https://github.com/firebase/firebase-ios-sdk", from: "11.0.0"),
    ],
    targets: [
        .target(
            name: "EncedoPushPlugin",
            dependencies: [
                .byName(name: "Tauri"),
                .product(name: "FirebaseMessaging", package: "firebase-ios-sdk"),
            ],
            path: "Sources/EncedoPushPlugin")
    ]
)
