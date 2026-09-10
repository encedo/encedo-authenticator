import Foundation
import Tauri
import UIKit

/// The storage key's protector on iOS. The Keychain holds a 32-byte wrapping key
/// that never leaves the device (`ThisDeviceOnly`, so it is not in iCloud or a
/// backup restored onto another phone); `wrap`/`unwrap` are AES-GCM around it,
/// the same contract the Android side offers over the hardware keystore.
///
/// Losing the key (device wiped, app data cleared) means pairing again, which is
/// what the plan chose over any migration.
class BytesArgs: Decodable {
  let data: String
}

class KeystorePlugin: Plugin {
  private let account = "encedo.authenticator.store.v2"
  private let service = "com.encedo.mobile.auth"

  private func key() throws -> SymmetricKeyBox {
    if let existing = try read() {
      return SymmetricKeyBox(bytes: existing)
    }
    var bytes = [UInt8](repeating: 0, count: 32)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
      throw KeystoreError.message("could not draw a key")
    }
    try write(Data(bytes))
    return SymmetricKeyBox(bytes: Data(bytes))
  }

  private func read() throws -> Data? {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess, let data = item as? Data else {
      throw KeystoreError.message("keychain read failed (\(status))")
    }
    return data
  }

  private func write(_ data: Data) throws {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecValueData as String: data,
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
    ]
    SecItemDelete(query as CFDictionary)
    let status = SecItemAdd(query as CFDictionary, nil)
    guard status == errSecSuccess else { throw KeystoreError.message("keychain write failed (\(status))") }
  }

  @objc public func wrap(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(BytesArgs.self)
    guard let plain = Data(base64Encoded: args.data) else { throw KeystoreError.message("argument is not base64") }
    let sealed = try key().seal(plain)
    invoke.resolve(["data": sealed.base64EncodedString()])
  }

  @objc public func unwrap(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(BytesArgs.self)
    guard let blob = Data(base64Encoded: args.data) else { throw KeystoreError.message("argument is not base64") }
    let plain = try key().open(blob)
    invoke.resolve(["data": plain.base64EncodedString()])
  }

  /// What the Manager shows as the paired phone's label.
  @objc public func deviceName(_ invoke: Invoke) throws {
    invoke.resolve(["name": UIDevice.current.name])
  }
}

enum KeystoreError: LocalizedError {
  case message(String)
  var errorDescription: String? {
    switch self {
    case .message(let m): return m
    }
  }
}

@_cdecl("init_plugin_encedo_keystore")
func initPlugin() -> Plugin {
  return KeystorePlugin()
}
