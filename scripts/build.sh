#!/bin/bash

# This script builds the Cordova project and should be called from within the container

# Exit on error
set -e

cd /app

export CI=true
cordova platform add android@14.0.1 || true
cordova build android --debug
cordova build android --release

# Create output directory
mkdir -p /app/output

# Copy APK and ABB fils to output directory
cp /app/platforms/android/app/build/outputs/apk/debug/app-debug.apk /app/output/
cp /app/platforms/android/app/build/outputs/bundle/release/app-release.aab /app/output/

echo "Done"