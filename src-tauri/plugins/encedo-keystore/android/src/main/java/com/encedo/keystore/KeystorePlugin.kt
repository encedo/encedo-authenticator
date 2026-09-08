package com.encedo.keystore

import android.app.Activity
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class BytesArg {
    var data: String = ""
}

/**
 * wrap(data)   -> nonce(12) || AES-256-GCM(data) under a Keystore key that never leaves the TEE.
 * unwrap(blob) -> data.
 * The key is created on first use; losing it (factory reset, app data cleared) means re-pairing.
 */
@TauriPlugin
class KeystorePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val ALIAS = "encedo.authenticator.store.v2"
        private const val PROVIDER = "AndroidKeyStore"
        private const val TRANSFORM = "AES/GCM/NoPadding"
    }

    private fun key(): SecretKey {
        val ks = KeyStore.getInstance(PROVIDER).apply { load(null) }
        (ks.getEntry(ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        gen.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .build()
        )
        return gen.generateKey()
    }

    private fun reply(invoke: Invoke, bytes: ByteArray) {
        val out = JSObject()
        out.put("data", Base64.encodeToString(bytes, Base64.NO_WRAP))
        invoke.resolve(out)
    }

    /** What the Manager shows as the paired phone's label. */
    @Command
    fun deviceName(invoke: Invoke) {
        val out = JSObject()
        val model = Build.MODEL ?: ""
        val maker = Build.MANUFACTURER ?: ""
        val name = if (model.lowercase().startsWith(maker.lowercase())) model else "$maker $model"
        out.put("name", name.trim().replaceFirstChar { it.uppercase() })
        invoke.resolve(out)
    }

    @Command
    fun wrap(invoke: Invoke) {
        try {
            val data = Base64.decode(invoke.parseArgs(BytesArg::class.java).data, Base64.NO_WRAP)
            val c = Cipher.getInstance(TRANSFORM)
            c.init(Cipher.ENCRYPT_MODE, key())
            reply(invoke, c.iv + c.doFinal(data))
        } catch (e: Exception) {
            invoke.reject("keystore wrap failed: ${e.message}")
        }
    }

    @Command
    fun unwrap(invoke: Invoke) {
        try {
            val blob = Base64.decode(invoke.parseArgs(BytesArg::class.java).data, Base64.NO_WRAP)
            if (blob.size < 12 + 16) throw IllegalArgumentException("blob too short")
            val c = Cipher.getInstance(TRANSFORM)
            c.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, blob, 0, 12))
            reply(invoke, c.doFinal(blob, 12, blob.size - 12))
        } catch (e: Exception) {
            invoke.reject("keystore unwrap failed: ${e.message}")
        }
    }
}
