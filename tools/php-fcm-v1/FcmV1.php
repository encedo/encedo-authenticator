<?php
/**
 * FCM HTTP v1 sender for the legacy notify backend (PHP 7.3, ext-openssl, ext-curl, ext-json).
 *
 * The legacy API (`/fcm/send`, `Authorization: key=…`) was switched off by Google
 * on 20 June 2024. v1 needs an OAuth2 access token minted from a service
 * account: sign a JWT with the account's RSA key, exchange it for a bearer
 * token valid for one hour, cache it, and mint a new one when it is about to
 * expire. No refresh token, no browser, no user.
 *
 * Usage:
 *   $fcm = new FcmV1('/etc/encedo/firebase-service-account.json', '/var/cache/encedo/fcm-token.json');
 *   $fcm->sendLegacy($legacyPayload);   // what the old code used to POST to /fcm/send
 *
 * Secrets: the service-account JSON is the only secret; keep it outside the web root, mode 0600.
 */
final class FcmV1
{
    private const TOKEN_URL = 'https://oauth2.googleapis.com/token';
    private const SCOPE = 'https://www.googleapis.com/auth/firebase.messaging';
    /** Requests expire in about two minutes; a push older than that is noise. */
    private const ANDROID_TTL = '120s';
    private const ANDROID_CHANNEL = 'encedo_requests';

    /** @var array<string,mixed> */
    private $account;
    /** @var string */
    private $cachePath;
    /** @var int */
    private $timeout;

    public function __construct(string $serviceAccountPath, string $tokenCachePath, int $timeoutSeconds = 10)
    {
        $json = @file_get_contents($serviceAccountPath);
        if ($json === false) {
            throw new RuntimeException("FcmV1: cannot read service account file");
        }
        $account = json_decode($json, true);
        foreach (['client_email', 'private_key', 'project_id', 'token_uri'] as $k) {
            if (empty($account[$k])) {
                throw new RuntimeException("FcmV1: service account file lacks $k");
            }
        }
        $this->account = $account;
        $this->cachePath = $tokenCachePath;
        $this->timeout = $timeoutSeconds;
    }

    // ---- public API -----------------------------------------------------------

    /**
     * Send one message in FCM v1 form. `$message` is the `message` object
     * (token, notification, data, android, apns). Returns the message name.
     * Throws FcmSendException; `isUnregistered()` tells you to drop the token.
     */
    public function send(array $message): string
    {
        $url = sprintf('https://fcm.googleapis.com/v1/projects/%s/messages:send', rawurlencode($this->account['project_id']));
        $body = json_encode(['message' => $message], JSON_UNESCAPED_SLASHES);
        [$status, $resp] = $this->post($url, $body, ['Authorization: Bearer ' . $this->accessToken(), 'Content-Type: application/json']);
        if ($status === 401) {
            // Token revoked or clock skew: mint a fresh one and try once more.
            [$status, $resp] = $this->post($url, $body, ['Authorization: Bearer ' . $this->accessToken(true), 'Content-Type: application/json']);
        }
        $decoded = json_decode($resp, true);
        if ($status >= 200 && $status < 300 && isset($decoded['name'])) {
            return $decoded['name'];
        }
        throw new FcmSendException($status, $decoded['error'] ?? ['message' => substr($resp, 0, 200)]);
    }

    /**
     * Send what the old code used to POST to the legacy endpoint:
     *   { "to": "<token>", "notification": {"title","body","sound"?}, "data": {"encedo": "<json string>", ...}, "priority"?: "high" }
     * Everything in `data` is coerced to strings, as v1 requires.
     */
    public function sendLegacy(array $legacy): string
    {
        $token = $legacy['to'] ?? null;
        if (!is_string($token) || $token === '') {
            throw new InvalidArgumentException('FcmV1: legacy payload has no "to" token');
        }
        $data = [];
        foreach ((array)($legacy['data'] ?? []) as $k => $v) {
            $data[(string)$k] = is_string($v) ? $v : json_encode($v, JSON_UNESCAPED_SLASHES);
        }
        $notification = null;
        if (!empty($legacy['notification']['title']) || !empty($legacy['notification']['body'])) {
            $notification = [
                'title' => (string)($legacy['notification']['title'] ?? ''),
                'body' => (string)($legacy['notification']['body'] ?? ''),
            ];
        }
        $sound = $legacy['notification']['sound'] ?? 'default';

        $message = ['token' => $token];
        if ($notification !== null) {
            $message['notification'] = $notification;
        }
        if ($data !== []) {
            $message['data'] = $data;
        }
        $message['android'] = [
            'priority' => 'high',
            'ttl' => self::ANDROID_TTL,
            'notification' => ['channel_id' => self::ANDROID_CHANNEL, 'sound' => $sound],
        ];
        $message['apns'] = [
            'headers' => ['apns-priority' => '10'],
            'payload' => ['aps' => ['sound' => $sound]],
        ];
        return $this->send($message);
    }

    // ---- OAuth2 ----------------------------------------------------------------

    /** Bearer token, from cache while it has more than five minutes left. */
    public function accessToken(bool $force = false): string
    {
        if (!$force) {
            $cached = @json_decode((string)@file_get_contents($this->cachePath), true);
            if (is_array($cached) && !empty($cached['access_token']) && ($cached['expires_at'] ?? 0) - time() > 300) {
                return $cached['access_token'];
            }
        }
        $now = time();
        $claims = [
            'iss' => $this->account['client_email'],
            'scope' => self::SCOPE,
            'aud' => $this->account['token_uri'] ?: self::TOKEN_URL,
            'iat' => $now - 10,       // small skew allowance
            'exp' => $now + 3600,
        ];
        $jwt = $this->signJwt($claims);
        [$status, $resp] = $this->post(
            $claims['aud'],
            http_build_query(['grant_type' => 'urn:ietf:params:oauth:grant-type:jwt-bearer', 'assertion' => $jwt]),
            ['Content-Type: application/x-www-form-urlencoded']
        );
        $tok = json_decode($resp, true);
        if ($status !== 200 || empty($tok['access_token'])) {
            throw new RuntimeException("FcmV1: token exchange failed ($status): " . substr($resp, 0, 200));
        }
        $expiresAt = $now + (int)($tok['expires_in'] ?? 3600);
        $this->writePrivate($this->cachePath, json_encode(['access_token' => $tok['access_token'], 'expires_at' => $expiresAt]));
        return $tok['access_token'];
    }

    private function signJwt(array $claims): string
    {
        $head = self::b64url(json_encode(['alg' => 'RS256', 'typ' => 'JWT']));
        $body = self::b64url(json_encode($claims));
        $input = "$head.$body";
        $key = openssl_pkey_get_private($this->account['private_key']);
        if ($key === false) {
            throw new RuntimeException('FcmV1: service account private key is not readable: ' . openssl_error_string());
        }
        $sig = '';
        if (!openssl_sign($input, $sig, $key, OPENSSL_ALGO_SHA256)) {
            throw new RuntimeException('FcmV1: openssl_sign failed: ' . openssl_error_string());
        }
        return $input . '.' . self::b64url($sig);
    }

    // ---- plumbing ----------------------------------------------------------------

    /** @return array{0:int,1:string} */
    private function post(string $url, string $body, array $headers): array
    {
        $ch = curl_init($url);
        curl_setopt_array($ch, [
            CURLOPT_POST => true,
            CURLOPT_POSTFIELDS => $body,
            CURLOPT_HTTPHEADER => $headers,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => $this->timeout,
            CURLOPT_CONNECTTIMEOUT => $this->timeout,
            CURLOPT_SSL_VERIFYPEER => true,
        ]);
        $resp = curl_exec($ch);
        if ($resp === false) {
            $err = curl_error($ch);
            curl_close($ch);
            throw new RuntimeException("FcmV1: curl: $err");
        }
        $status = (int)curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
        curl_close($ch);
        return [$status, (string)$resp];
    }

    private function writePrivate(string $path, string $content): void
    {
        $dir = dirname($path);
        if (!is_dir($dir)) {
            @mkdir($dir, 0700, true);
        }
        $tmp = $path . '.' . getmypid() . '.tmp';
        if (file_put_contents($tmp, $content, LOCK_EX) === false) {
            return; // a missing cache only costs one extra token exchange
        }
        @chmod($tmp, 0600);
        @rename($tmp, $path);
    }

    private static function b64url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}

final class FcmSendException extends RuntimeException
{
    /** @var int */
    public $status;
    /** @var array<string,mixed> */
    public $error;

    public function __construct(int $status, array $error)
    {
        $this->status = $status;
        $this->error = $error;
        parent::__construct(sprintf('FCM v1 %d: %s', $status, $error['message'] ?? 'unknown'), $status);
    }

    /** The device token is gone (app uninstalled, token rotated): delete it from the subscriber. */
    public function isUnregistered(): bool
    {
        if ($this->status === 404) {
            return true;
        }
        foreach ((array)($this->error['details'] ?? []) as $d) {
            if (($d['errorCode'] ?? '') === 'UNREGISTERED') {
                return true;
            }
        }
        return false;
    }
}
