#!/usr/bin/env bash
# Production build for Google Play: package com.encedo.mobile.auth.android,
# signed with the upload key from keystore.jks. Run on vostro after rsync.
# Needs, outside the repo, on vostro:
#   ~/secrets/encedo-authenticator/keystore.jks
#   ~/secrets/encedo-authenticator/keystore.properties  (storeFile=..., storePassword=..., keyAlias=encedoauth, keyPassword=...)
#   ~/develop/encedo-authenticator/src-tauri/gen/android/app/src/main/res/values/firebase.xml (scripts/firebase-res.py)
# Do not install this over v1 on a phone that must keep v1: same package name.
set -euo pipefail
source ~/.android-env.sh
cd ~/develop/encedo-authenticator
SECRETS=~/secrets/encedo-authenticator
[ -f "$SECRETS/keystore.properties" ] || { echo "missing $SECRETS/keystore.properties"; exit 1; }
[ -f src-tauri/gen/android/app/src/main/res/values/firebase.xml ] || { echo "missing firebase.xml: run scripts/firebase-res.py"; exit 1; }
cp "$SECRETS/keystore.properties" src-tauri/gen/android/keystore.properties
trap 'rm -f src-tauri/gen/android/keystore.properties' EXIT
cargo tauri android build --aab --target aarch64 --target armv7 "$@"
ls -la src-tauri/gen/android/app/build/outputs/bundle/universalRelease/*.aab
