package com.encedo.keystore

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.security.keystore.UserNotAuthenticatedException
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
    /** Wrap under the key that only works right after the person confirmed who they are. */
    var auth: Boolean = false
}

/**
 * The storage key's protector. Two keys live in the Android Keystore, neither
 * exportable, both in StrongBox where the phone has it and both refusing to work
 * while the screen is locked:
 *
 *  - `…v3`      plain: usable whenever the app runs.
 *  - `…v3.auth` bound to the person: usable only within [AUTH_WINDOW_SECONDS]
 *               of a strong biometric or the device credential. Root can ask for
 *               a decryption as the app, but not without that confirmation — the
 *               authentication token is signed inside the TEE.
 *
 * `wrap` says which of the two it used, so `unwrap` needs no state:
 *   "EK2" | flags(1) | nonce(12) | AES-256-GCM ciphertext
 * A blob without the prefix is the first format (plain, alias `…v2`); the Rust
 * side re-wraps it on the next write.
 */
@TauriPlugin
class KeystorePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val LEGACY_ALIAS = "encedo.authenticator.store.v2"
        private const val PLAIN_ALIAS = "encedo.authenticator.store.v3"
        private const val AUTH_ALIAS = "encedo.authenticator.store.v3.auth"
        private const val PROVIDER = "AndroidKeyStore"
        private const val TRANSFORM = "AES/GCM/NoPadding"
        private val MAGIC = byteArrayOf('E'.code.toByte(), 'K'.code.toByte(), '2'.code.toByte())
        private const val FLAG_AUTH = 1
        /** Long enough for the prompt and the file read, short enough to mean something. */
        private const val AUTH_WINDOW_SECONDS = 30
    }

    private fun keystore(): KeyStore = KeyStore.getInstance(PROVIDER).apply { load(null) }

    private fun hasCredential(): Boolean {
        val km = activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager ?: return false
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) km.isDeviceSecure else km.isKeyguardSecure
    }

    private fun hasStrongBox(): Boolean =
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.P &&
            activity.packageManager.hasSystemFeature(PackageManager.FEATURE_STRONGBOX_KEYSTORE)

    private fun spec(alias: String, auth: Boolean, strongBox: Boolean): KeyGenParameterSpec {
        val b = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setRandomizedEncryptionRequired(true)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            // Nothing is decrypted while the phone is locked, whoever asks.
            b.setUnlockedDeviceRequired(true)
            if (strongBox) {
                b.setIsStrongBoxBacked(true)
            }
        }
        if (auth) {
            b.setUserAuthenticationRequired(true)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                // The credential counts as well as biometrics: a new fingerprint
                // then does not lock the person out of their own modules.
                b.setUserAuthenticationParameters(
                    AUTH_WINDOW_SECONDS,
                    KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                )
            } else {
                @Suppress("DEPRECATION")
                b.setUserAuthenticationValidityDurationSeconds(AUTH_WINDOW_SECONDS)
            }
        }
        return b.build()
    }

    /** The key, made on first use. StrongBox is asked for and given up on quietly. */
    private fun key(alias: String, auth: Boolean): SecretKey {
        (keystore().getEntry(alias, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        if (auth && !hasCredential()) {
            throw NoCredentialException()
        }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        val strongBox = hasStrongBox()
        try {
            gen.init(spec(alias, auth, strongBox))
            return gen.generateKey()
        } catch (e: Exception) {
            val unavailable = Build.VERSION.SDK_INT >= Build.VERSION_CODES.P && e is StrongBoxUnavailableException
            if (!strongBox || !unavailable) {
                throw e
            }
        }
        gen.init(spec(alias, auth, false))
        return gen.generateKey()
    }

    private fun reply(invoke: Invoke, bytes: ByteArray) {
        val out = JSObject()
        out.put("data", Base64.encodeToString(bytes, Base64.NO_WRAP))
        invoke.resolve(out)
    }

    /** Codes the Rust side matches on, so the app can say what to do next. */
    private fun fail(invoke: Invoke, e: Exception) {
        val code = when {
            e is UserNotAuthenticatedException -> "auth_required"
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.M && e is KeyPermanentlyInvalidatedException -> "key_lost"
            e is NoCredentialException -> "no_credential"
            else -> "keystore"
        }
        invoke.reject("$code: ${e.message ?: e.javaClass.simpleName}")
    }

    private class NoCredentialException : Exception("this phone has no screen lock, so a key cannot be bound to you")

    /** What this phone can protect a key with. */
    @Command
    fun protection(invoke: Invoke) {
        val out = JSObject()
        out.put("credential", hasCredential())
        out.put("strongBox", hasStrongBox())
        out.put("windowSeconds", AUTH_WINDOW_SECONDS)
        invoke.resolve(out)
    }

    @Command
    fun wrap(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(BytesArg::class.java)
            val data = Base64.decode(args.data, Base64.NO_WRAP)
            val alias = if (args.auth) AUTH_ALIAS else PLAIN_ALIAS
            val c = Cipher.getInstance(TRANSFORM)
            c.init(Cipher.ENCRYPT_MODE, key(alias, args.auth))
            val flags = if (args.auth) FLAG_AUTH else 0
            reply(invoke, MAGIC + byteArrayOf(flags.toByte()) + c.iv + c.doFinal(data))
        } catch (e: Exception) {
            fail(invoke, e)
        }
    }

    @Command
    fun unwrap(invoke: Invoke) {
        try {
            val blob = Base64.decode(invoke.parseArgs(BytesArg::class.java).data, Base64.NO_WRAP)
            val tagged = blob.size > MAGIC.size && blob.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)
            val head = if (tagged) MAGIC.size + 1 else 0
            val auth = tagged && (blob[MAGIC.size].toInt() and FLAG_AUTH) != 0
            val alias = if (!tagged) LEGACY_ALIAS else if (auth) AUTH_ALIAS else PLAIN_ALIAS
            if (blob.size < head + 12 + 16) {
                throw IllegalArgumentException("blob too short")
            }
            if ((keystore().getEntry(alias, null) as? KeyStore.SecretKeyEntry) == null) {
                // The file is here, its key is not: a restored backup, or the lock
                // was removed. Nothing can read this store again.
                throw KeyMissingException()
            }
            val c = Cipher.getInstance(TRANSFORM)
            c.init(Cipher.DECRYPT_MODE, key(alias, auth), GCMParameterSpec(128, blob, head, 12))
            reply(invoke, c.doFinal(blob, head + 12, blob.size - head - 12))
        } catch (e: Exception) {
            fail(invoke, e)
        }
    }

    private class KeyMissingException : Exception("the key that protects this store is gone from this phone")

    /** Whether the blob was wrapped under the key bound to the person. */
    @Command
    fun boundToUser(invoke: Invoke) {
        val blob = Base64.decode(invoke.parseArgs(BytesArg::class.java).data, Base64.NO_WRAP)
        val tagged = blob.size > MAGIC.size && blob.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)
        val out = JSObject()
        out.put("bound", tagged && (blob[MAGIC.size].toInt() and FLAG_AUTH) != 0)
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
}
