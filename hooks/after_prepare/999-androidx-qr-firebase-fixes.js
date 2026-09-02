#!/usr/bin/env node
const fs = require('fs');

function fix(file, replacements) {
  if (!fs.existsSync(file)) return;
  let s = fs.readFileSync(file, 'utf8');
  let t = s;
  for (const [from, to] of replacements) t = t.replace(from, to);
  if (t !== s) fs.writeFileSync(file, t);
}

// QRScanner → AndroidX
fix('platforms/android/app/src/main/java/com/bitpay/cordova/qrscanner/QRScanner.java', [
  [/android\.support\.v4\.app\.ActivityCompat/g, 'androidx.core.app.ActivityCompat'],
  [/android\.support\.v4\.content\.ContextCompat/g, 'androidx.core.content.ContextCompat']
]);

// Firebasex ambiguous bigLargeIcon(null)
fix('platforms/android/app/src/main/java/org/apache/cordova/firebase/FirebasePluginMessagingService.java', [
  [/bigLargeIcon\(null\)/g, 'bigLargeIcon((android.graphics.Bitmap) null)']
]);

// Ensure AndroidX flags
const gp = 'platforms/android/gradle.properties';
if (fs.existsSync(gp)) {
  let txt = fs.readFileSync(gp, 'utf8');
  if (!/android.useAndroidX=true/.test(txt)) txt += '\nandroid.useAndroidX=true\n';
  if (!/android.enableJetifier=true/.test(txt)) txt += 'android.enableJetifier=true\n';
  fs.writeFileSync(gp, txt);
}

// Firebasex: remove internal Crashlytics import that no longer exists
fix('platforms/android/app/src/main/java/org/apache/cordova/firebase/FirebasePlugin.java', [
  [/^import\s+com\.google\.firebase\.crashlytics\.internal\.metadata\.UserMetadata;\s*$/m, ''],
]);