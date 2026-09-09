package com.encedo.push

import android.Manifest
import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.os.Build
import android.util.Log
import android.webkit.WebView
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.RemoteMessage

/**
 * Commands: getToken, requestPermissions, checkPermissions (base class).
 * Events: "token" {token}, "message" {title, body, data}, "tapped" {data},
 * "lifecycle" {state: "paused" | "resumed"} (the app lock and the refresh on return hang on it).
 */
@TauriPlugin(
    permissions = [
        Permission(strings = [Manifest.permission.POST_NOTIFICATIONS], alias = "notifications")
    ]
)
class PushPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        const val CHANNEL = "encedo_requests"
        @Volatile var instance: PushPlugin? = null
        private val systemKeys = setOf(
            "from", "collapse_key", "message_id", "google.message_id", "google.sent_time",
            "google.ttl", "google.original_priority", "google.delivered_priority", "gcm.n.analytics_data",
            "profile", "android.intent.extra.REFERRER"
        )
    }

    override fun load(webView: WebView) {
        instance = this
        ensureChannel()
        deliverTap(activity.intent)
    }

    /** The channel the backend addresses (`android.notification.channel_id`). A
     *  notification for a channel that does not exist is dropped on Android 8+. */
    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val nm = activity.getSystemService(NotificationManager::class.java) ?: return
        if (nm.getNotificationChannel(CHANNEL) != null) return
        val ch = NotificationChannel(CHANNEL, "Access requests", NotificationManager.IMPORTANCE_HIGH)
        ch.description = "A module asks this phone to allow or deny an operation"
        nm.createNotificationChannel(ch)
    }

    override fun onNewIntent(intent: Intent) {
        deliverTap(intent)
    }

    private var pausedAt = 0L

    // The webview is frozen while the activity is paused, so anything sent on
    // pause is only read on resume; the time away is therefore measured here.
    override fun onPause() {
        pausedAt = System.currentTimeMillis()
        lifecycle("paused", 0)
    }

    override fun onResume() {
        val away = if (pausedAt > 0) System.currentTimeMillis() - pausedAt else 0
        lifecycle("resumed", away)
    }

    private fun lifecycle(state: String, awayMs: Long) {
        Log.i("EncedoPush", "lifecycle $state away=$awayMs listeners=${hasListener("lifecycle")}")
        val payload = JSObject()
        payload.put("state", state)
        payload.put("awayMs", awayMs)
        trigger("lifecycle", payload)
    }

    @Command
    fun getToken(invoke: Invoke) {
        FirebaseMessaging.getInstance().token.addOnCompleteListener { task ->
            if (task.isSuccessful && !task.result.isNullOrEmpty()) {
                val result = JSObject()
                result.put("token", task.result)
                invoke.resolve(result)
            } else {
                invoke.reject("FCM token unavailable: " + (task.exception?.message ?: "empty"))
            }
        }
    }

    override fun requestPermissions(invoke: Invoke) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            getPermissionState("notifications") == PermissionState.GRANTED
        ) {
            invoke.resolve(state())
            return
        }
        requestPermissionForAlias("notifications", invoke, "notificationsCallback")
    }

    @PermissionCallback
    fun notificationsCallback(invoke: Invoke) {
        invoke.resolve(state())
    }

    private fun state(): JSObject {
        val s = JSObject()
        val granted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            getPermissionState("notifications") == PermissionState.GRANTED
        s.put("notifications", if (granted) "granted" else "denied")
        return s
    }

    fun onToken(token: String) {
        val payload = JSObject()
        payload.put("token", token)
        trigger("token", payload)
    }

    fun onMessage(message: RemoteMessage) {
        val payload = JSObject()
        message.notification?.let {
            payload.put("title", it.title)
            payload.put("body", it.body)
        }
        val data = JSObject()
        for ((k, v) in message.data) data.put(k, v)
        payload.put("data", data)
        trigger("message", payload)
    }

    /** A system-shown notification opens the launcher intent with the data keys as extras. */
    private fun deliverTap(intent: Intent?) {
        val extras = intent?.extras ?: return
        val data = JSObject()
        var any = false
        for (k in extras.keySet()) {
            if (k in systemKeys) continue
            val v = extras.get(k) ?: continue
            data.put(k, v.toString())
            any = true
        }
        if (!any) return
        val payload = JSObject()
        payload.put("data", data)
        trigger("tapped", payload)
        intent.replaceExtras(android.os.Bundle())
    }
}
