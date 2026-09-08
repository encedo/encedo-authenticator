// Native plugin access, in one place, with browser fallbacks so the mockup
// still runs in a plain tab. Keys and the protocol stay in Rust; this module
// only moves display data and a push token.

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface PushState {
  status: "unavailable" | "pending" | "registered" | "denied" | "error";
  token?: string;
  permission?: "granted" | "denied" | "unknown";
  error?: string;
}

export interface PushMessage {
  /** Unix seconds. */
  at: number;
  title?: string;
  body?: string;
  data: Record<string, unknown>;
  tapped?: boolean;
}

/** Scan one QR code with the camera behind a transparent webview. Returns the
 *  content, or null when the user cancelled or the camera is not available. */
export async function scanQr(zoom?: number): Promise<string | null> {
  if (!inTauri) return null;
  const s = await import("@tauri-apps/plugin-barcode-scanner");
  let p = await s.checkPermissions();
  if (p !== "granted" && p !== "denied") p = await s.requestPermissions();
  if (p !== "granted") throw new Error("camera permission denied");
  document.documentElement.classList.add("scanning");
  try {
    const r = await s.scan({ windowed: true, formats: [s.Format.QRCode], cameraDirection: "back", zoom } as Parameters<typeof s.scan>[0]);
    return r.content;
  } catch (e) {
    if (String(e).toLowerCase().includes("cancel")) return null;
    throw e;
  } finally {
    document.documentElement.classList.remove("scanning");
  }
}

export async function cancelScan() {
  if (!inTauri) return;
  const s = await import("@tauri-apps/plugin-barcode-scanner");
  await s.cancel().catch(() => {});
  document.documentElement.classList.remove("scanning");
}

export interface ZoomRange { min: number; max: number; current: number }

/** The lens's zoom range while the scanner runs; null before the camera is bound. */
export async function scanZoomRange(): Promise<ZoomRange | null> {
  if (!inTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  try { return await invoke<ZoomRange>("plugin:barcode-scanner|zoom_range"); } catch { return null; }
}

export async function setScanZoom(ratio: number): Promise<number | null> {
  if (!inTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  try { return (await invoke<{ ratio: number }>("plugin:barcode-scanner|set_zoom", { ratio })).ratio; } catch { return null; }
}

export async function openAppSettings() {
  if (!inTauri) return;
  const s = await import("@tauri-apps/plugin-barcode-scanner");
  await s.openAppSettings();
}

type PushHandlers = {
  onToken: (token: string) => void;
  onMessage: (m: PushMessage) => void;
};

const PUSH = "encedo-push";

type PermissionMap = { notifications: "granted" | "denied" | "prompt" | "prompt-with-rationale" };

/** Attach FCM listeners and fetch the current token. Safe to call once at start. */
export async function startPush(h: PushHandlers): Promise<PushState> {
  if (!inTauri) return { status: "unavailable" };
  const { invoke, addPluginListener } = await import("@tauri-apps/api/core");
  const toMessage = (raw: unknown, tapped = false): PushMessage => {
    const r = (raw ?? {}) as Record<string, unknown>;
    return {
      at: new Date().toISOString(),
      title: r.title as string | undefined,
      body: r.body as string | undefined,
      data: (r.data as Record<string, unknown>) ?? {},
      tapped,
    };
  };
  try {
    await addPluginListener<{ token: string }>(PUSH, "token", ({ token }) => h.onToken(token));
    await addPluginListener(PUSH, "message", (m) => h.onMessage(toMessage(m)));
    await addPluginListener(PUSH, "tapped", (m) => h.onMessage(toMessage(m, true)));
  } catch (e) {
    return { status: "error", error: "listeners: " + String(e) };
  }
  let permission: PushState["permission"] = "unknown";
  try {
    const p = await invoke<PermissionMap>(`plugin:${PUSH}|check_permissions`);
    permission = p.notifications === "granted" ? "granted" : p.notifications === "denied" ? "denied" : "unknown";
  } catch { /* older Android: no runtime permission */ }
  try {
    const { token } = await invoke<{ token: string }>(`plugin:${PUSH}|get_token`);
    return { status: "registered", token, permission };
  } catch (e) {
    return { status: "error", error: String(e), permission };
  }
}

export async function requestPushPermission(): Promise<boolean> {
  if (!inTauri) return false;
  const { invoke } = await import("@tauri-apps/api/core");
  const p = await invoke<PermissionMap>(`plugin:${PUSH}|request_permissions`);
  return p.notifications === "granted";
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand("copy");
    ta.remove();
    return ok;
  }
}
