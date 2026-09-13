#!/usr/bin/env bash
# Put the app on an iPhone with a free Apple ID (a "personal team").
#
# What a free team can and cannot do:
#   - it signs an app for 7 days, then the app stops launching until rebuilt;
#   - it cannot hold the push entitlement, so this build has no push at all
#     (requests still arrive: the Now screen asks the broker every 15 s);
#   - it cannot claim a bundle id owned by another team, hence the .dev suffix.
#
# Before the first run, on the Mac (Screen Sharing: vnc://macmini):
#   1. Xcode → Settings → Accounts → + → Apple ID → sign in.
#      The team id is next to "Personal Team"; `security find-identity -v -p codesigning`
#      shows the certificate once Xcode has made one.
#   2. Plug the iPhone in, unlock it, tap "Trust This Computer".
#   3. On the iPhone: Settings → Privacy & Security → Developer Mode → on, then reboot.
# After the first install, on the iPhone: Settings → General → VPN & Device Management
# → trust the developer, or the app refuses to open.
#
#   scripts/macmini-ios-dev.sh <TEAM_ID>
set -euo pipefail

# codesign reads the signing key from the login keychain, which an ssh session
# may not open ("User interaction is not allowed"). The Mac auto-logs in, so its
# GUI session has the keychain unlocked: step into it and carry on there.
if [ -n "${SSH_CONNECTION:-}" ] && [ -z "${ENCEDO_GUI_SESSION:-}" ]; then
  if ! security show-keychain-info ~/Library/Keychains/login.keychain-db >/dev/null 2>&1; then
    export ENCEDO_GUI_SESSION=1
    exec sudo launchctl asuser "$(id -u)" sudo -u "$(whoami)" \
      env ENCEDO_GUI_SESSION=1 APPLE_DEVELOPMENT_TEAM="${1:-${APPLE_DEVELOPMENT_TEAM:-}}" \
      bash "$0" "$@"
  fi
fi
[ -f ~/.zshenv ] && . ~/.zshenv 2>/dev/null

TEAM="${1:-${APPLE_DEVELOPMENT_TEAM:-}}"
if [ -z "$TEAM" ]; then
  echo "usage: macmini-ios-dev.sh <TEAM_ID>   (Xcode → Settings → Accounts → Personal Team)" >&2
  exit 2
fi

SRC=~/develop/encedo-authenticator
DEV=~/develop/encedo-authenticator-ios-dev
COUNTER=~/.encedo-ios-build
N=$(( $(cat "$COUNTER" 2>/dev/null || echo 0) + 1 )); echo "$N" > "$COUNTER"

# A copy, so the main project keeps the production bundle id and the push entitlement.
rsync -a --delete --exclude node_modules --exclude 'src-tauri/target' \
  --exclude 'src-tauri/crates/*/target' --exclude 'src-tauri/gen/android' \
  --exclude 'src-tauri/gen/apple/build' "$SRC/" "$DEV/"
cd "$DEV"
npm install --no-audit --no-fund >/dev/null
npm run build >/dev/null

cat > src-tauri/tauri.ios.conf.json <<JSON
{
  "identifier": "com.encedo.mobile.auth.ios.dev",
  "productName": "Encedo HEM Auth dev",
  "version": "2.0.0-dev.$N",
  "bundle": { "iOS": { "developmentTeam": "$TEAM" } }
}
JSON

rm -rf src-tauri/gen/apple
npm run tauri -- ios init --ci
cp "$SRC/src-tauri/gen/apple/tauri" src-tauri/gen/apple/tauri          # CLI shim the Xcode build phase needs
mkdir -p src-tauri/gen/apple/assets
cp "$SRC/src-tauri/gen/apple/assets/GoogleService-Info.plist" src-tauri/gen/apple/assets/ 2>/dev/null || true
# The square iOS icons are already generated in the repo; the Mac has no PIL.
cp -R "$SRC/src-tauri/gen/apple/Assets.xcassets/." src-tauri/gen/apple/Assets.xcassets/ 2>/dev/null || true
# A personal team cannot sign an app that asks for push.
for f in src-tauri/gen/apple/*/*.entitlements; do
  /usr/libexec/PlistBuddy -c "Delete :aps-environment" "$f" 2>/dev/null || true
done

export APPLE_DEVELOPMENT_TEAM="$TEAM"
npm run tauri -- ios build --debug --target aarch64

# `tauri ios build` exports an .ipa; older versions leave a plain .app.
APP=$(ls -d src-tauri/gen/apple/build/arm64/*.ipa src-tauri/gen/apple/build/arm64/*.app 2>/dev/null | head -1)
[ -n "$APP" ] || { echo "nothing to install in build/arm64" >&2; exit 1; }
echo "built: $APP  (dev build $N, bundle com.encedo.mobile.auth.ios.dev)"

UDID=$(xcrun xctrace list devices 2>/dev/null | awk '/^iPhone .*\(.*\) \(/ {gsub(/[()]/,"",$NF); print $NF; exit}' || true)
if [ -n "$UDID" ]; then
  xcrun devicectl device install app --device "$UDID" "$APP" | tail -3
  echo
  echo "On the iPhone, once per certificate: Settings → General → VPN & Device Management"
  echo "→ Apple Development: <your Apple ID> → Trust. Until then iOS refuses to launch it."
  echo "Then: xcrun devicectl device process launch --device $UDID com.encedo.mobile.auth.ios.dev"
else
  echo "no iPhone connected; plug it in and run: xcrun devicectl device install app --device <udid> \"$APP\""
fi
