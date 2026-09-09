<?php
/**
 * End-to-end check of FCM v1 from any host with PHP 7.3+ (openssl, curl, json):
 *
 *   php test-send.php /path/to/service-account.json '<device token>' ['<data.encedo json>'] [--silent]
 *
 * Default payload is a v1-shaped event, so the app refreshes from the broker.
 * Prints the message name from Google or the error.
 */
require_once __DIR__ . '/FcmV1.php';

$args = array_values(array_filter(array_slice($argv, 1), function ($a) { return $a !== '--silent'; }));
$silent = in_array('--silent', $argv, true);
if (count($args) < 2) {
    fwrite(STDERR, "usage: php test-send.php <service-account.json> <device-token> [<data.encedo json>] [--silent]\n");
    exit(2);
}
[$account, $token] = $args;
$encedo = $args[2] ?? json_encode(['event' => ['pid' => 'test', 'source' => 'test-send.php']]);

$fcm = new FcmV1($account, sys_get_temp_dir() . '/encedo-fcm-token.json');
$msg = [
    'token' => $token,
    'data' => ['encedo' => $encedo, 'notification_foreground' => 'true'],
    'android' => ['priority' => 'high', 'ttl' => '300s', 'notification' => ['channel_id' => 'encedo_requests', 'sound' => 'default']],
    'apns' => ['headers' => ['apns-priority' => '10'], 'payload' => ['aps' => ['sound' => 'default']]],
];
if (!$silent) {
    $msg['notification'] = ['title' => 'Encedo Mobile Authenticator', 'body' => 'Test message from test-send.php'];
} else {
    unset($msg['android']['notification']);
}
try {
    echo "sent: ", $fcm->send($msg), "\n";
} catch (FcmSendException $e) {
    echo "FCM refused ({$e->status}): ", json_encode($e->error), "\n";
    if ($e->isUnregistered()) echo "token is not registered: reinstall the app or copy a fresh token\n";
    exit(1);
} catch (RuntimeException $e) {
    echo "failed: ", $e->getMessage(), "\n";
    exit(1);
}
