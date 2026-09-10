#!/usr/bin/env bash
# What still stands between the repo and the app running on an iPhone.
# Read-only; run it over SSH: ssh macmini 'bash ~/develop/encedo-authenticator/scripts/macmini-ios-check.sh'
[ -f ~/.zshenv ] && . ~/.zshenv 2>/dev/null

ok()   { printf '  \033[32mok\033[0m    %s\n' "$1"; }
todo() { printf '  \033[33mtodo\033[0m  %s\n' "$1"; }

# Xcode's settings and the signing key live in the logged-in session; an ssh
# session sees neither, so every probe below goes through it.
gui() { sudo launchctl asuser "$(id -u)" sudo -u "$(whoami)" "$@" 2>/dev/null; }

echo "Xcode"
if xcodebuild -version >/dev/null 2>&1; then ok "$(xcodebuild -version | head -1) at $(xcode-select -p)"; else todo "no Xcode"; fi
if xcrun simctl list runtimes 2>/dev/null | grep -q "iOS 26"; then ok "iOS simulator runtime present"; else todo "xcodebuild -downloadPlatform iOS"; fi

echo "Apple account (the one step that needs the GUI: Xcode → Settings → Accounts)"
TEAMS=$(gui defaults read com.apple.dt.Xcode IDEProvisioningTeamByIdentifier | grep -E "teamID|teamName|isFreeProvisioningTeam")
if [ -n "$TEAMS" ]; then
  ok "account signed in:"
  printf '%s\n' "$TEAMS" | sed 's/^ *//' | sed 's/^/        /'
else
  todo "no Apple ID in Xcode — sign in over Screen Sharing (vnc://macmini)"
fi

echo "Signing"
IDS=$(gui security find-identity -v -p codesigning | grep -c "Apple Development")
[ "${IDS:-0}" -gt 0 ] && ok "$IDS development certificate(s)" || todo "no signing certificate yet — Xcode makes one during the first signed build"
PROFILES=$(ls ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision 2>/dev/null | wc -l | tr -d ' ')
[ "${PROFILES:-0}" -gt 0 ] && ok "$PROFILES provisioning profile(s)" || todo "no provisioning profile yet — a free team needs a registered device first"

echo "Keychain from this session"
if security show-keychain-info ~/Library/Keychains/login.keychain-db >/dev/null 2>&1; then
  ok "login keychain reachable"
elif sudo launchctl asuser "$(id -u)" sudo -u "$(whoami)" security show-keychain-info ~/Library/Keychains/login.keychain-db >/dev/null 2>&1; then
  ok "locked for ssh, reachable through the logged-in session (the build script re-enters it)"
else
  todo "login keychain locked; unlock it in the GUI session or sign in again"
fi

echo "iPhone"
DEVICES=$(xcrun devicectl list devices 2>/dev/null | grep -viE "^Devices|^Name|^--|^$|No devices found" | head -3)
if [ -n "$DEVICES" ]; then
  ok "connected:"; printf '%s\n' "$DEVICES" | sed 's/^/        /'
else
  todo "no device — plug the iPhone in by cable, unlock it, trust the computer"
  todo "on the phone: Settings → Privacy & Security → Developer Mode → on, then reboot"
fi

echo
TEAM=$(gui defaults read com.apple.dt.Xcode IDEProvisioningTeamByIdentifier | sed -n 's/.*teamID = \([A-Z0-9]*\).*/\1/p' | head -1)
if [ -n "$TEAM" ]; then
  echo "Next:  scripts/macmini-ios-dev.sh $TEAM"
else
  echo "Next:  sign in to Xcode, then run this again"
fi
