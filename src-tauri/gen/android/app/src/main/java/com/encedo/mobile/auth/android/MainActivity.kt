package com.encedo.mobile.auth.android

import android.os.Bundle
import android.view.View
import android.view.WindowManager
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.lifecycle.ProcessLifecycleOwner

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Tauri 2.11 generates TauriLifecycleObserver but never registers it, so
    // plugins never hear onPause/onResume. Process-level, like wry's own: the
    // app going to the background counts, our own dialogs (biometric prompt,
    // permission requests) do not.
    ProcessLifecycleOwner.get().lifecycle.addObserver(TauriLifecycleObserver)

    // The webview does not know about system bars; keep it (and the camera
    // preview behind it) inside them. The bars show the window background,
    // which the theme paints in the app's ground colour.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { v, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
      v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      insets
    }

    // What this app shows is a security decision: no screenshots, no recents
    // thumbnail. Dev builds (package .dev) stay capturable for test screenshots.
    if (!packageName.endsWith(".dev")) {
      window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
    }
  }
}
