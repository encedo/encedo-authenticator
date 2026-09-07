package com.encedo.push

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

/** Firebase calls this on its own thread; the plugin forwards to the webview. */
class FCMService : FirebaseMessagingService() {
    override fun onMessageReceived(message: RemoteMessage) {
        PushPlugin.instance?.onMessage(message)
    }

    override fun onNewToken(token: String) {
        PushPlugin.instance?.onToken(token)
    }
}
