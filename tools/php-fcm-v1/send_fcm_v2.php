<?php
// ---- paste from here into the old script -------------------------------------
// FCM HTTP v1 replacement for send_fcm(). PHP 7.3+, ext-openssl, ext-curl, ext-json.
// Service account: with nothing set, the first *.json file next to this script
// that contains "service_account" is used. $FCM_V2_SERVICE_ACCOUNT may also be
// set beforehand to a file path, a JSON string or the decoded array.

if (!isset($FCM_V2_SERVICE_ACCOUNT)) {
    $FCM_V2_SERVICE_ACCOUNT = null;
    foreach ((array)glob(__DIR__ . '/*.json') as $f) {
        if (strpos((string)@file_get_contents($f), '"service_account"') !== false) { $FCM_V2_SERVICE_ACCOUNT = $f; break; }
    }
}

/** The service account as an array, whatever form $FCM_V2_SERVICE_ACCOUNT takes. */
function fcm_v2_service_account() {
    global $FCM_V2_SERVICE_ACCOUNT;
    static $sa = null;
    if ($sa !== null) { return $sa; }
    $v = $FCM_V2_SERVICE_ACCOUNT;
    if (is_string($v) && $v !== '' && $v[0] !== '{' && is_file($v)) { $v = file_get_contents($v); }
    $sa = is_array($v) ? $v : json_decode((string)$v, true);
    if (empty($sa['client_email']) || empty($sa['private_key'])) {
        throw new RuntimeException('fcm_v2: no service account: put the Firebase service-account .json next to this script or set $FCM_V2_SERVICE_ACCOUNT');
    }
    return $sa;
}

/** Bearer token for firebase.messaging, minted from the service account and cached for its lifetime. */
function fcm_v2_access_token($force = false) {
    static $mem = null;
    $cache = sys_get_temp_dir() . '/encedo-fcm-v2-token.json';
    if (!$force) {
        if ($mem === null) { $mem = @json_decode((string)@file_get_contents($cache), true); }
        if (is_array($mem) && !empty($mem['access_token']) && ($mem['expires_at'] - time()) > 300) {
            return $mem['access_token'];
        }
    }
    $sa = fcm_v2_service_account();
    $tokenUrl = !empty($sa['token_uri']) ? $sa['token_uri'] : 'https://oauth2.googleapis.com/token';
    $now = time();
    $b64 = function ($s) { return rtrim(strtr(base64_encode($s), '+/', '-_'), '='); };
    $input = $b64(json_encode(['alg' => 'RS256', 'typ' => 'JWT'])) . '.' . $b64(json_encode([
        'iss' => $sa['client_email'],
        'scope' => 'https://www.googleapis.com/auth/firebase.messaging',
        'aud' => $tokenUrl,
        'iat' => $now - 10,
        'exp' => $now + 3600,
    ]));
    $key = openssl_pkey_get_private($sa['private_key']);
    if ($key === false) { throw new RuntimeException('fcm_v2: private key unreadable: ' . openssl_error_string()); }
    $sig = '';
    if (!openssl_sign($input, $sig, $key, OPENSSL_ALGO_SHA256)) { throw new RuntimeException('fcm_v2: openssl_sign failed'); }
    $jwt = $input . '.' . $b64($sig);

    $ch = curl_init($tokenUrl);
    curl_setopt_array($ch, [
        CURLOPT_POST => true,
        CURLOPT_POSTFIELDS => http_build_query(['grant_type' => 'urn:ietf:params:oauth:grant-type:jwt-bearer', 'assertion' => $jwt]),
        CURLOPT_HTTPHEADER => ['Content-Type: application/x-www-form-urlencoded'],
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_TIMEOUT => 10,
    ]);
    $resp = curl_exec($ch);
    $status = (int)curl_getinfo($ch, CURLINFO_HTTP_CODE);
    $err = curl_error($ch);
    curl_close($ch);
    $tok = json_decode((string)$resp, true);
    if ($status !== 200 || empty($tok['access_token'])) {
        throw new RuntimeException("fcm_v2: token exchange failed ($status) $err " . substr((string)$resp, 0, 200));
    }
    $mem = ['access_token' => $tok['access_token'], 'expires_at' => $now + (int)($tok['expires_in'] ?? 3600)];
    @file_put_contents($cache, json_encode($mem), LOCK_EX);
    @chmod($cache, 0600);
    return $mem['access_token'];
}

/** Same signature, Redis keys and return shape as the old send_fcm(); transport is FCM v1. */
function send_fcm_v2($devid, $data, $silent = false) {
    try { $sa = fcm_v2_service_account(); } catch (RuntimeException $ex) { $sa = []; }
    $project = isset($sa['project_id']) ? $sa['project_id'] : 'encedo-mobile-authenticator';

    $msg = [
        'token' => $devid,
        'data' => [
            'encedo' => is_string($data) ? $data : json_encode($data, JSON_UNESCAPED_SLASHES),  // v1: every data value is a string
            'notification_foreground' => 'true',
        ],
        'android' => [
            'collapse_key' => 't',
            'priority' => 'high',
            'ttl' => $silent === true ? '3600s' : '300s',
            'notification' => ['channel_id' => 'encedo_requests', 'color' => '#0F5F4B', 'sound' => 'default'],
        ],
        'apns' => [
            'headers' => ['apns-priority' => '10'],
            'payload' => ['aps' => ['badge' => 1, 'sound' => 'default']],
        ],
    ];
    if ($silent === true) {
        unset($msg['android']['notification']);
        $msg['apns'] = ['headers' => ['apns-priority' => '5', 'apns-push-type' => 'background'], 'payload' => ['aps' => ['content-available' => 1]]];
    } else {
        $msg['notification'] = ['title' => 'Encedo Mobile Authenticator', 'body' => 'New event from Encedo application'];
    }

    $out = ['status' => 0, 'response' => null, 'msg' => $msg];
    $url = 'https://fcm.googleapis.com/v1/projects/' . rawurlencode($project) . '/messages:send';
    $body = json_encode(['message' => $msg], JSON_UNESCAPED_SLASHES);
    $post = function ($token) use ($url, $body) {
        $ch = curl_init($url);
        curl_setopt_array($ch, [
            CURLOPT_POST => true,
            CURLOPT_POSTFIELDS => $body,
            CURLOPT_HTTPHEADER => ['Content-Type: application/json', 'Authorization: Bearer ' . $token],
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => 10,
        ]);
        $resp = curl_exec($ch);
        $status = (int)curl_getinfo($ch, CURLINFO_HTTP_CODE);
        $err = curl_error($ch);
        curl_close($ch);
        return [$status, (string)$resp, $err];
    };

    try {
        list($status, $resp, $err) = $post(fcm_v2_access_token());
        if ($status === 401) { list($status, $resp, $err) = $post(fcm_v2_access_token(true)); }
        $decoded = json_decode($resp, true);
        $out['status'] = $status;
        if ($status >= 200 && $status < 300 && isset($decoded['name'])) {
            $out['response'] = ['success' => 1, 'failure' => 0, 'results' => [['message_id' => $decoded['name']]], 'v1' => $decoded];
        } elseif ($status === 0) {
            $out['response'] = ['success' => 0, 'failure' => 0, 'results' => [], 'error' => 'curl: ' . $err];  // network: not the device's fault
        } else {
            $e = isset($decoded['error']) ? $decoded['error'] : ['message' => substr($resp, 0, 200)];
            $unregistered = ($status === 404);
            foreach ((array)($e['details'] ?? []) as $d) { if (($d['errorCode'] ?? '') === 'UNREGISTERED') { $unregistered = true; } }
            // NotRegistered is what the old cleanup looked for.
            $out['response'] = ['success' => 0, 'failure' => 1, 'results' => [['error' => $unregistered ? 'NotRegistered' : ($e['status'] ?? 'InternalServerError')]], 'v1' => $e];
        }
    } catch (RuntimeException $ex) {
        $out['status'] = 0;
        $out['response'] = ['success' => 0, 'failure' => 0, 'results' => [], 'error' => $ex->getMessage()];
    }

    $key = 'fcm:txs:' . ($devid);
    $redis = new Redis();
    $redis->pconnect('127.0.0.1', 6379);
    $redis->hset($key, getmicrotime(), json_encode($out));
    if (isset($out['response']['failure']) && $out['response']['failure'] > 0) {
        $redis->rpush('fcm:failures', json_encode($out));
        //$jobId = Resque::enqueue('default', 'FCM_CleanUp', $out, true);
    }
    return $out;
}
// ---- paste up to here ---------------------------------------------------------
