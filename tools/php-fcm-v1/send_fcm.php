<?php
/**
 * Drop-in replacement for the legacy send_fcm($devid, $data, $silent) in the
 * notify backend. Same signature, same Redis bookkeeping (`fcm:txs:<token>`,
 * `fcm:failures`), same return shape; only the transport changes to FCM v1.
 *
 * Differences from the old message, on purpose:
 *   - priority high and a short TTL for requests (they expire on the module in
 *     minutes anyway); silent messages (pairing DELETED) keep an hour;
 *   - Android channel `encedo_requests` (the v2 app creates it; v1 used the
 *     default channel);
 *   - `data.encedo` is always a string, as v1 requires for every data value.
 *
 * The response is normalised to the legacy `{success, failure, results}` so the
 * existing failure handling keeps working; the raw v1 answer is under `v1`.
 */
require_once __DIR__ . '/FcmV1.php';

function send_fcm($devid, $data, $silent = false)
{
    static $fcm = null;
    if ($fcm === null) {
        $fcm = new FcmV1(
            getenv('ENCEDO_FCM_SERVICE_ACCOUNT') ?: '/etc/encedo/firebase-service-account.json',
            getenv('ENCEDO_FCM_TOKEN_CACHE') ?: '/var/cache/encedo/fcm-token.json'
        );
    }

    $encedo = is_string($data) ? $data : json_encode($data, JSON_UNESCAPED_SLASHES);

    $msg = [
        'token' => $devid,
        'data' => [
            'encedo' => $encedo,
            'notification_foreground' => 'true',
        ],
        'android' => [
            'collapse_key' => 't',
            'priority' => 'high',
            'ttl' => $silent === true ? '3600s' : '300s',
            'notification' => [
                'channel_id' => 'encedo_requests',
                'color' => '#0F5F4B',
                'sound' => 'default',
            ],
        ],
        'apns' => [
            'headers' => ['apns-priority' => '10'],
            'payload' => ['aps' => ['badge' => 1, 'sound' => 'default']],
        ],
    ];
    if ($silent !== true) {
        $msg['notification'] = [
            'title' => 'Encedo Mobile Authenticator',
            'body' => 'New event from Encedo application',
        ];
    } else {
        // Data-only: nothing for the system to show, so no notification section at all.
        unset($msg['android']['notification'], $msg['apns']['payload']['aps']['sound'], $msg['apns']['payload']['aps']['badge']);
        $msg['apns']['payload']['aps']['content-available'] = 1;
        $msg['apns']['headers']['apns-priority'] = '5';
        $msg['apns']['headers']['apns-push-type'] = 'background';
    }

    $out = ['status' => 0, 'response' => null, 'msg' => $msg];
    try {
        $name = $fcm->send($msg);
        $out['status'] = 200;
        $out['response'] = ['success' => 1, 'failure' => 0, 'results' => [['message_id' => $name]], 'v1' => ['name' => $name]];
    } catch (FcmSendException $e) {
        $out['status'] = $e->status;
        $out['response'] = [
            'success' => 0,
            'failure' => 1,
            // Legacy error names the old cleanup understood: NotRegistered means "drop this token".
            'results' => [['error' => $e->isUnregistered() ? 'NotRegistered' : ($e->error['status'] ?? 'InternalServerError')]],
            'v1' => $e->error,
        ];
    } catch (RuntimeException $e) {
        // Token exchange or network: not the device's fault, do not clean it up.
        $out['status'] = 0;
        $out['response'] = ['success' => 0, 'failure' => 0, 'results' => [], 'error' => $e->getMessage()];
    }

    $key = 'fcm:txs:' . $devid;
    $redis = new Redis();
    $redis->pconnect('127.0.0.1', 6379);
    $redis->hset($key, getmicrotime(), json_encode($out));
    if (($out['response']['failure'] ?? 0) > 0) {
        $redis->rpush('fcm:failures', json_encode($out));
        //$jobId = Resque::enqueue('default', 'FCM_CleanUp', $out, true);
    }

    return $out;
}
