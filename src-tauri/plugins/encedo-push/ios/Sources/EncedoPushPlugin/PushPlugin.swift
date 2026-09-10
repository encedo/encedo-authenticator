import FirebaseCore
import FirebaseMessaging
import Foundation
import Tauri
import UIKit
import UserNotifications
import WebKit

/// Push on iOS, the way v1 did it: APNs delivers, Firebase exchanges the APNs
/// token for an FCM registration token, and the backend keeps addressing one
/// kind of token for both platforms.
///
/// Commands: getToken, requestPermissions, checkPermissions.
/// Events: "token" {token}, "message" {title, body, data}, "tapped" {data},
/// "lifecycle" {state, awayMs} — the app lock and the refresh on return hang on it.
class PushPlugin: Plugin {
  private var pausedAt: Date?
  private var pendingTap: JSObject?

  public override func load(webview: WKWebView) {
    configureFirebase()
    Messaging.messaging().delegate = self
    UNUserNotificationCenter.current().delegate = self
    // Registering is safe before permission: iOS only delivers once the person agrees.
    DispatchQueue.main.async { UIApplication.shared.registerForRemoteNotifications() }
    observeLifecycle()
  }

  /// The Xcode project copies `assets/` as a folder reference, so the file is
  /// not at the bundle root and `FirebaseApp.configure()` alone would not find it.
  private func configureFirebase() {
    guard FirebaseApp.app() == nil else { return }
    let root = Bundle.main.path(forResource: "GoogleService-Info", ofType: "plist")
    let inAssets = Bundle.main.path(forResource: "GoogleService-Info", ofType: "plist", inDirectory: "assets")
    guard let path = root ?? inAssets, let options = FirebaseOptions(contentsOfFile: path) else {
      Logger.error("encedo-push: no GoogleService-Info.plist in the bundle; push is off")
      return
    }
    FirebaseApp.configure(options: options)
  }

  // ---- commands --------------------------------------------------------------

  @objc public func getToken(_ invoke: Invoke) {
    Messaging.messaging().token { token, error in
      if let token = token, !token.isEmpty {
        invoke.resolve(["token": token])
      } else {
        invoke.reject("FCM token unavailable: \(error?.localizedDescription ?? "empty")")
      }
    }
  }

  @objc public override func requestPermissions(_ invoke: Invoke) {
    UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound]) { granted, error in
      if let error = error {
        invoke.reject("notification permission: \(error.localizedDescription)")
        return
      }
      if granted {
        DispatchQueue.main.async { UIApplication.shared.registerForRemoteNotifications() }
      }
      invoke.resolve(["notifications": granted ? "granted" : "denied"])
    }
  }

  @objc public override func checkPermissions(_ invoke: Invoke) {
    UNUserNotificationCenter.current().getNotificationSettings { settings in
      let state: String
      switch settings.authorizationStatus {
      case .authorized, .provisional, .ephemeral: state = "granted"
      case .denied: state = "denied"
      default: state = "prompt"
      }
      invoke.resolve(["notifications": state])
    }
  }

  // ---- events ----------------------------------------------------------------

  fileprivate func emitMessage(_ userInfo: [AnyHashable: Any], event: String) {
    // Everything crossing to the webview is a string, as it is on Android:
    // `data.encedo` is the JSON the core parses.
    var data = JSObject()
    for (k, v) in userInfo {
      guard let key = k as? String, !key.hasPrefix("gcm."), !key.hasPrefix("google."), key != "aps" else { continue }
      data[key] = "\(v)"
    }
    var payload: JSObject = ["data": data]
    if let aps = userInfo["aps"] as? [String: Any], let alert = aps["alert"] as? [String: Any] {
      payload["title"] = alert["title"] as? String ?? ""
      payload["body"] = alert["body"] as? String ?? ""
    }
    trigger(event, data: payload)
  }

  private func observeLifecycle() {
    let centre = NotificationCenter.default
    centre.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
      self?.pausedAt = Date()
      self?.trigger("lifecycle", data: ["state": "paused", "awayMs": 0] as JSObject)
    }
    centre.addObserver(forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main) { [weak self] _ in
      guard let self = self else { return }
      let away = self.pausedAt.map { Int(Date().timeIntervalSince($0) * 1000) } ?? 0
      self.pausedAt = nil
      self.trigger("lifecycle", data: ["state": "resumed", "awayMs": away] as JSObject)
      if let tap = self.pendingTap {
        self.pendingTap = nil
        self.trigger("tapped", data: tap)
      }
    }
  }
}

extension PushPlugin: MessagingDelegate {
  func messaging(_ messaging: Messaging, didReceiveRegistrationToken fcmToken: String?) {
    guard let token = fcmToken, !token.isEmpty else { return }
    trigger("token", data: ["token": token] as JSObject)
  }
}

extension PushPlugin: UNUserNotificationCenterDelegate {
  /// A message while the app is in front: show nothing, hand it to the webview.
  func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification,
                              withCompletionHandler handler: @escaping (UNNotificationPresentationOptions) -> Void) {
    emitMessage(notification.request.content.userInfo, event: "message")
    handler([.banner, .sound])
  }

  /// The person tapped the notification.
  func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
                              withCompletionHandler handler: @escaping () -> Void) {
    emitMessage(response.notification.request.content.userInfo, event: "tapped")
    handler()
  }
}

@_cdecl("init_plugin_encedo_push")
func initPlugin() -> Plugin {
  return PushPlugin()
}
