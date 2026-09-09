# FcmV1.php — patch for the legacy notify backend

Drop-in sender for FCM HTTP v1 on PHP 7.3 (ext-openssl, ext-curl, ext-json), no
Composer. Replaces the POST to `https://fcm.googleapis.com/fcm/send` that
Google switched off on 20 June 2024.

1. In the Firebase console (project `encedo-mobile-authenticator`) → Project
   settings → Service accounts → Generate new private key. Put the JSON outside
   the web root, e.g. `/etc/encedo/firebase-service-account.json`, mode 0600,
   owned by the PHP user.
2. Pick a writable cache path for the access token, e.g.
   `/var/cache/encedo/fcm-token.json` (created on first use).
3. Where the old code built the legacy payload and posted it:

```php
require_once __DIR__ . '/FcmV1.php';
$fcm = new FcmV1('/etc/encedo/firebase-service-account.json', '/var/cache/encedo/fcm-token.json');
try {
    $fcm->sendLegacy($payload);           // the same array the old code posted
} catch (FcmSendException $e) {
    if ($e->isUnregistered()) {
        // the phone's token is dead: remove this subscriber's fid
    }
    error_log($e->getMessage());
} catch (RuntimeException $e) {
    error_log($e->getMessage());          // token exchange or network
}
```

How the token works: the service account's RSA key signs a JWT (`iss` =
client_email, `scope` = firebase.messaging, `aud` = token_uri, one hour), one
POST to `oauth2.googleapis.com/token` returns a bearer token for an hour; it is
cached in the file and minted again with five minutes to spare, or at once on
a 401 from FCM. Nothing else to renew.

What `sendLegacy` changes on the way to v1: every `data` value becomes a
string (`data.encedo` already is one), Android gets `priority: high`, a 120 s
TTL and the `encedo_requests` channel, iOS gets `apns-priority: 10` and the
sound. The rest of the payload is passed as is.
