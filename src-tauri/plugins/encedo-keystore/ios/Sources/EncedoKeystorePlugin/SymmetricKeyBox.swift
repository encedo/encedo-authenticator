import CryptoKit
import Foundation

/// AES-256-GCM with the Keychain key: `nonce(12) || ciphertext || tag(16)`,
/// the same layout the Android half writes, so a blob is readable by whichever
/// side made it on that platform.
struct SymmetricKeyBox {
  let key: SymmetricKey

  init(bytes: Data) {
    self.key = SymmetricKey(data: bytes)
  }

  func seal(_ plain: Data) throws -> Data {
    let sealed = try AES.GCM.seal(plain, using: key)
    guard let combined = sealed.combined else { throw KeystoreError.message("could not seal") }
    return combined
  }

  func open(_ blob: Data) throws -> Data {
    let box = try AES.GCM.SealedBox(combined: blob)
    return try AES.GCM.open(box, using: key)
  }
}
