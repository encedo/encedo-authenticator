# Welcome to Encedo Manager
Let's make privacy private again


## About Encedo

Encedo is a secure enclave and trusted execution environment, the source of root-of-trust in the cloud. Encedo is a secure place to store and use cryptographic keys used to protect blockchain apps (wallets), encrypted emails or encrypted cloud storage. Encedo Manager is your eyes and ears to control Encedo enclave.

Encedo Manager allows secure, transactions based pairing between Encedo enclave and mobile device (phone or tablet). Every time Encedo needs to validate access token, in the background sends a request to all paired devices with a simple question: Do you allow or deny this access?

Encedo Manager provides remote governance over how your secure enclave is used.

## Demo .apk

Debug version of last build is available here:
http://olgroup.usermd.net/app-debug-test-0-0-7.apk

## Plugins used

Here is the list of all plugins used in this application

### Screenshot prevention
Allow/Disallow the creation of screenshots dinamically with JS. It will make your app gray in the recent apps view.
https://sdkcarlos.github.io/sites/cordova/disable-screenshots.html

### QR Scanner
A fast, energy efficient, highly-configurable QR code scanner for Cordova apps – available for the iOS, Android, Windows, and browser platforms.
https://www.npmjs.com/package/cordova-plugin-qrscanner

### Splashscreen
This plugin displays and hides a splash screen while your web application is launching. Using its methods you can also show and hide the splash screen manually.
https://github.com/apache/cordova-plugin-splashscreen

### SQLite Storage
Native SQLite component with API based on HTML5/Web SQL (DRAFT) API.
https://www.npmjs.com/package/cordova-sqlite-storage

### Crypto with Minisodium
A minimal build of the libsodium library, as a plugin for Cordova applications on iOS and Android.
https://github.com/LockateMe/cordova-plugin-minisodium

### Firebase Cloud Messaging
Send a push notification to a single device or topic.
https://www.npmjs.com/package/cordova-plugin-fcm-with-dependecy-updated

### Fingerprint Authentication
This plugin provides a single and simple interface for accessing fingerprint APIs on both Android 6+ and iOS.
https://www.npmjs.com/package/cordova-plugin-fingerprint-aio

___

### (tested, removed) Fingerprint Authentication for Android
This plugin was created referencing the Fingerprint Dialog sample and the Confirm Credential sample referenced by the Android 6.0 APIs webpage.
https://github.com/mjwheatley/cordova-plugin-android-fingerprint-auth

### (tested, removed) Google Firebase Cloud Messaging Cordova Push Plugin
https://github.com/fechanique/cordova-plugin-fcm

### (tested, removed) Google Firebase Cloud Messaging Cordova Push Plugin
https://www.npmjs.com/package/cordova-plugin-fcm-ng

## Building from scratch

If you would like to create a new project with Encedo Manager, just follow steps that are listed below one after another. 

* cordova create encedoApp com.encedo.mobile.auth.android encedoApp
* cd encedoApp
* cordova platform add ios 
* cordova platform add android 
* cordova build android --debug (build of blank project for testing purposes)

* cordova plugin add https://github.com/sdkcarlos/cordova-ourcodeworld-preventscreenshots.git
* cordova plugin add cordova-plugin-qrscanner
* cordova plugin add cordova-plugin-splashscreen
* cordova plugin add cordova-sqlite-storage
* cordova plugin add cordova-plugin-minisodium
* cordova plugin add cordova-plugin-fcm-with-dependecy-updated --variable PAGE_LINK_DOMAIN="https://notify.encedo.com/test_fcm/"
* cordova plugin add cordova-plugin-fingerprint-aio

If you open existing project that has been created a while ago please remember about updating all plugins and dependencies. You can make it by simply typing those commands:

* cordova platform update android --save
* cordova platform update ios --save

## Credits
All tools, plugins and elements that have been used to create this Application are free to use (also in commercial way) without need for crediting the Autors. However we would like to say "Thank you" for all the creators and people of the internet that helped us with their work.

### Plugins
We would like to thanks people like Carlos Delgado, ematiu, gasteve, nitsujlangston, brodybits, LockateMe, andretissot, niklasmerz for contributing to cordova/ionic enviroment and making this project easier to handle on both platforms.

### Artwork 
Those beatiful images has been created by PixelTrue and can be found here:
https://www.pixeltrue.com/free-packs/adventure-illustrations

These images are shared on free license (Free to use for personal and commercial use without attribution):
https://www.pixeltrue.com/license

### Fonts
Font used in application is "Inter" and can be found here:
https://fonts.google.com/specimen/Inter?preview.text_type=custom

These fonts are licensed under the Open Font License. 
They can be used freely in your products & projects - print or digital, commercial or otherwise. 
More about the OFL can be found here:
https://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL

### Icons
Icon set has been created using Fontello (fontello.com) mostly with Font Awesome icons.
This is the same license that works with Inter Font (listed above) and another source of information about it you can find here:
https://en.wikipedia.org/wiki/SIL_Open_Font_License

