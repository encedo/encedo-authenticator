import Foundation
import LocalAuthentication
import Tauri
import UIKit

/// The storage key's protector on iOS, the same contract the Android side offers
/// over the hardware keystore. Two Keychain items can hold the 32-byte wrapping
/// key, both `ThisDeviceOnly` so neither is in iCloud or in a backup restored
/// onto another phone:
///
///  - `…v3`      plain: readable while the phone is unlocked.
///  - `…v3.auth` behind `.userPresence`: every read asks for Face ID, Touch ID or
///               the passcode, so a phone under someone else's control does not
///               give up the key without the person.
///
/// `wrap` writes which one it used into the blob ("EK2" | flags | sealed), so
/// `unwrap` needs no state of its own. A blob without that prefix is the first
/// format, kept under `…v2`; the Rust side re-wraps it on the next write.
class BytesArgs: Decodable {
  let data: String
  let auth: Bool?
}

class KeystorePlugin: Plugin {
  private let legacyAccount = "encedo.authenticator.store.v2"
  private let plainAccount = "encedo.authenticator.store.v3"
  private let authAccount = "encedo.authenticator.store.v3.auth"
  private let service = "com.encedo.mobile.auth"
  private let magic = Array("EK2".utf8)
  private let flagAuth: UInt8 = 1

  // ---- keychain ------------------------------------------------------------

  private func read(_ account: String) throws -> Data? {
    var query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    if account == authAccount {
      // This shows the system prompt, which is why every command runs off the
      // main thread.
      query[kSecUseOperationPrompt as String] = "Confirm it is you to open your modules"
    }
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    if status == errSecItemNotFound { return nil }
    if status == errSecUserCanceled || status == errSecAuthFailed || status == errSecInteractionNotAllowed {
      throw KeystoreError.coded("auth_required", "not confirmed, so the storage key stays sealed")
    }
    guard status == errSecSuccess, let data = item as? Data else {
      throw KeystoreError.coded("keystore", "keychain read failed (\(status))")
    }
    return data
  }

  private func write(_ data: Data, _ account: String) throws {
    var query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecValueData as String: data,
    ]
    if account == authAccount {
      guard hasCredential() else {
        throw KeystoreError.coded("no_credential", "this phone has no passcode, so a key cannot be bound to you")
      }
      var error: Unmanaged<CFError>?
      guard let access = SecAccessControlCreateWithFlags(nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, .userPresence, &error) else {
        throw KeystoreError.coded("keystore", "could not ask for your presence")
      }
      query[kSecAttrAccessControl as String] = access
    } else {
      query[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    }
    SecItemDelete([
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
    ] as CFDictionary)
    let status = SecItemAdd(query as CFDictionary, nil)
    guard status == errSecSuccess else {
      throw KeystoreError.coded("keystore", "keychain write failed (\(status))")
    }
  }

  private func hasCredential() -> Bool {
    var error: NSError?
    return LAContext().canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
  }

  /// The wrapping key for this account, made on first use.
  private func key(_ account: String) throws -> SymmetricKeyBox {
    if let existing = try read(account) {
      return SymmetricKeyBox(bytes: existing)
    }
    var bytes = [UInt8](repeating: 0, count: 32)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
      throw KeystoreError.coded("keystore", "could not draw a key")
    }
    try write(Data(bytes), account)
    return SymmetricKeyBox(bytes: Data(bytes))
  }

  // ---- blob header ---------------------------------------------------------

  private func tagged(_ blob: Data) -> Bool {
    blob.count > magic.count && Array(blob.prefix(magic.count)) == magic
  }

  private func boundFlag(_ blob: Data) -> Bool {
    tagged(blob) && (blob[blob.startIndex + magic.count] & flagAuth) != 0
  }

  // ---- commands ------------------------------------------------------------

  @objc public func wrap(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(BytesArgs.self)
    DispatchQueue.global(qos: .userInitiated).async {
      do {
        guard let plain = Data(base64Encoded: args.data) else {
          throw KeystoreError.coded("keystore", "argument is not base64")
        }
        let bound = args.auth ?? false
        let sealed = try self.key(bound ? self.authAccount : self.plainAccount).seal(plain)
        var blob = Data(self.magic)
        blob.append(bound ? self.flagAuth : 0)
        blob.append(sealed)
        invoke.resolve(["data": blob.base64EncodedString()])
      } catch {
        invoke.reject(KeystoreError.text(error))
      }
    }
  }

  @objc public func unwrap(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(BytesArgs.self)
    DispatchQueue.global(qos: .userInitiated).async {
      do {
        guard let blob = Data(base64Encoded: args.data) else {
          throw KeystoreError.coded("keystore", "argument is not base64")
        }
        let isTagged = self.tagged(blob)
        let bound = self.boundFlag(blob)
        let account = !isTagged ? self.legacyAccount : bound ? self.authAccount : self.plainAccount
        guard let stored = try self.read(account) else {
          throw KeystoreError.coded("key_lost", "the key that protects this store is gone from this phone")
        }
        let body = isTagged ? Data(blob.dropFirst(self.magic.count + 1)) : blob
        let plain = try SymmetricKeyBox(bytes: stored).open(body)
        invoke.resolve(["data": plain.base64EncodedString()])
      } catch {
        invoke.reject(KeystoreError.text(error))
      }
    }
  }

  /// What this phone can protect a key with. `windowSeconds` is zero because iOS
  /// asks on every use instead of opening a window after one confirmation.
  @objc public func protection(_ invoke: Invoke) throws {
    invoke.resolve([
      "credential": hasCredential(),
      "strongBox": LAContext().biometryType != .none,
      "windowSeconds": 0,
    ])
  }

  @objc public func boundToUser(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(BytesArgs.self)
    let blob = Data(base64Encoded: args.data) ?? Data()
    invoke.resolve(["bound": boundFlag(blob)])
  }

  /// What the Manager shows as the paired phone's label.
  @objc public func deviceName(_ invoke: Invoke) throws {
    invoke.resolve(["name": UIDevice.current.name])
  }
}

enum KeystoreError: LocalizedError {
  case message(String)
  case coded(String, String)

  var errorDescription: String? {
    switch self {
    case .message(let m): return m
    case .coded(let code, let m): return "\(code): \(m)"
    }
  }

  /// The Rust side reads `code: message`; anything else is a plain keystore fault.
  static func text(_ error: Error) -> String {
    if let k = error as? KeystoreError, let described = k.errorDescription {
      return described
    }
    return "keystore: \(error.localizedDescription)"
  }
}

@_cdecl("init_plugin_encedo_keystore")
func initPlugin() -> Plugin {
  return KeystorePlugin()
}
