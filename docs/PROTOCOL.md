# Protokół notify z v1 — do przeniesienia 1:1

Zapis z kodu v1 (branch `main`): `www/js/index.js`, `www/js/base.js`, `www/js/scopes.js`. Wektory testowe
z fazy 0 rozstrzygają każdą wątpliwość co do kodowania. Poprawki dla v2 oznaczone jako **[v2]**.

## Klucze i identyfikatory

| Nazwa | Co to jest | Kodowanie |
|---|---|---|
| `eid` | klucz publiczny X25519 managera (EncedoID), `iss` w żądaniu parowania | base64 standardowe, 32 B |
| `aid` / `aid_prv` | para X25519 aplikacji, generowana przy parowaniu (`axlsign.generateKeyPair(random32)`) | base64, 32 B każdy |
| `epk` | efemeryczny klucz publiczny sesji (brokera); inny przy parowaniu, przy każdym zdarzeniu i przy `/notify/session` | base64, 32 B |
| `pid` | pairing id nadany przez serwer | base64 standardowe |
| `pidx` | `pid` w ścieżce URL: `/` → `_`, `+` → `-`, bez `=` | base64url bez paddingu |
| `jti` | id żądania; **bajty** (`Base64.parse(jti)`) są wejściem HMAC przy wyprowadzaniu klucza sesji | base64 |
| `fid` | token push (FCM); w v1 zmienna `tokenNOW`, wartość domyślna to placeholder gdy brak uprawnień | string |
| `label` | `device.model + " (" + device.platform + ")"` | string |

Prymitywy: X25519 przez `axlsign.sharedKey(prv, pub)` (Curve25519, klucz prywatny = 32 losowe bajty po
clampingu; `x25519-dalek::StaticSecret::from(bytes)` daje ten sam wynik), HMAC-SHA256 (CryptoJS), AES-128-CBC
z paddingiem PKCS7 (domyślne CryptoJS), JWT HS256 pisany ręcznie:

```
header  = {"ecdh":"x25519","alg":"HS256","typ":"JWT"}     # kolejność pól jak w v1
jwt     = b64url(utf8(json(header))) "." b64url(utf8(json(payload))) "." b64url(HMAC-SHA256(head "." body, secret))
secret  = 32 B wspólnego sekretu X25519 (bajty, nie base64)
```

Sygnatura liczona jest po zakodowanym stringu, więc dowolna kolejność pól byłaby poprawna kryptograficznie;
trzymamy kolejność z v1, żeby wektory z v1 pasowały bajt w bajt.

## Przepływ 1: parowanie (`parseQRCode`, `genReply`)

1. QR z kamery: JSON `{link, user, hostname, email}`.
2. `GET link` → `{request: <JWT>, ipinfo_eid: {city, country, ip}}`. Payload JWT (bez weryfikacji podpisu,
   aplikacja nie ma jeszcze sekretu): `{jti, exp, iss: eid, aud: epk}` (tak wystawia moduł, potwierdzone
   przykładem z docs.encedo.com 2026-09-09). v1 czytało `eat`, którego nie ma, i `JSON.stringify` gubił
   `undefined`, więc odpowiedź v1 nie zawiera `eat`. **[v2]** `eat` opcjonalne i przepisywane jak przyszło;
   `exp` sprawdzane; `hash` z kodu QR (`base64(SHA-256(request))`, Manager v1 i SDK) porównywany z `request`.
3. Jeśli `eid` jest już w magazynie: „Already paired”, `DELETE link`, koniec.
4. Generujemy `aid`/`aid_prv`. `S1 = X25519(aid_prv, eid)`.
5. `reply = JWT_HS256(S1, {jti, eat, iss: aid, aud: eid, epk, label})`.
6. `mac = base64(HMAC-SHA256(utf8(reply + fid), X25519(aid_prv, epk)))`.
7. Użytkownik potwierdza (ekran z `user`, `hostname`, `email`, geolokalizacja wystawcy).
   Zgoda: `POST link {reply, fid, mac, aid}` → `{pid}`. Odmowa: `DELETE link` z tym samym body.
8. Zapis urządzenia: `{pid, eid, aid_prv, aid, iat, datetime, user, email, host}`.

Strona HEM (docs.encedo.com): `POST /api/auth/ext/init {epk}` → `{request, eid}`, potem
`POST /api/auth/ext/validate {pid, reply}` → `{kid, code}`; wymaga scope `auth:ext:pair` lub `system:config`.

## Przepływ 2: żądanie dostępu (`checkEventsToHandle`, `executeEvent`)

Wyzwalacze odpytania: push, tap w powiadomienie, `resume`, `focus`, odblokowanie, zakończenie poprzedniego
zdarzenia. Jedno zdarzenie naraz (`_eventBeingHandled`).

1. `POST /notify/event/data/allbypid {pid: [wszystkie pid z magazynu]}` → `{eventid: {<eventId>: [<pid>], ...}}`
   (na żywo 2026-09-09: wartość to jednoelementowa tablica; v1 robiło `String(value)`, co ją spłaszcza; pusta
   lista zdarzeń przychodzi jako tablica, v1 sprawdza `!Array.isArray`).
2. Dla pierwszego zdarzenia: `GET /notify/event/data/{eventId}/{pidx}` →
   `{jti, scope, epk, exp, ipinfo_eid: {city, country, ip}}`.
3. Wygaśnięcie: jeśli `now > exp`, `DELETE` zdarzenia, wpis w archiwum, następne. **[v2]** sprawdzać przed
   pokazaniem ekranu; w v1 ta gałąź odwołuje się do niezdefiniowanych zmiennych i wywala się cicho.
4. Dekrypcja scope:
   ```
   raw   = base64decode(scope[1:])          # scope[0] == "A" to identyfikator algorytmu
   ct    = raw[0 : len-32] ; mac32 = raw[len-32 :]
   S1    = X25519(aid_prv, eid)
   M     = HMAC-SHA256(bytes(base64decode(jti)), S1)      # 32 B, klucz sesji
   key   = M[0:16] ; iv = M[16:32]
   plain = AES-128-CBC-decrypt(ct, key, iv) z PKCS7      # string UTF-8, np. "keymgmt:use:<kid>#<b64>"
   ```
   **[v2]** `HMAC-SHA256(utf8(plain), M) == mac32` w stałym czasie, inaczej zdarzenie odrzucone i wpis
   w archiwum. v1 liczy obie wartości i tylko je loguje.
5. Dopasowanie scope do tabeli (`_scopes` w `scopes.js`): najpierw równość, potem pierwszy pasujący regex
   z kluczy tabeli; grupy z `match` trafiają do `%1..%n` w tekstach. Dla `keymgmt:use:<kid>#<b64json>`
   `b64json` to `{t: "<hex typu klucza>", l: "<label>"}`, a typ jest tłumaczony przez `keytype2string`
   (`base.js`). Scope spoza tabeli: wpis w archiwum, zdarzenie oznaczone jako obsłużone, powrót do home.
6. Ekran decyzji: nagłówek i opis ze scope, geolokalizacja wystawcy, opcje: okres dostępu 15 min (domyślnie),
   1 h, 8 h, 24 h; dla `storage:*` przełącznik „Permission to write” (`:rw`).
7. Zgoda:
   ```
   scope' = scope bez ":rw" (+ ":rw" jeśli przełącznik włączony)
   enc    = "A" + base64( AES-128-CBC-encrypt(utf8(scope'), key, iv) || HMAC-SHA256(utf8(scope'), M) )
   exp'   = now + okres
   reply  = JWT_HS256(S1, {aud: eid, jti, exp: exp', iat: now, pid, iss: aid, scope: enc})
   mac    = base64(HMAC-SHA256(utf8(reply), X25519(aid_prv, epk)))     # epk z tego zdarzenia
   POST /notify/event/data/{eventId}/{pidx} {authreply: reply, mac}
   ```
   Odmowa: `DELETE /notify/event/data/{eventId}/{pidx}`.
8. Po odpowiedzi zdarzenie trafia do lokalnej tabeli obsłużonych i następuje kolejne odpytanie.

Strona HEM: `POST /api/auth/ext/request {epk, scope, exp}` → `{authreq, epk}` (to trafia do notify),
`POST /api/auth/ext/token {authreply}` → `{token}`.

Mapowanie odpowiedzi serwera (`handleEventError`):

| Kod | Znaczenie w v1 |
|---|---|
| 401 | błąd krytyczny |
| 404 | zdarzenie wygasło |
| 410 + `{reason: "cancelled"}` | anulowane przez użytkownika po stronie HEM |
| 410 | obsłużone przez inną aplikację |
| ≥ 500 | usługa niedostępna |
| inne | żądanie odrzucone |

## Przepływ 3: unpair z aplikacji (`unpairDevice`)

```
POST /notify/session {aid}            → {epk}
nonce = random 32 B
mac   = base64(HMAC-SHA256(bytes(nonce), X25519(aid_prv, epk)))     # wejście HMAC to bajty nonce
POST /notify/subscribers/delete {pid, epk, nonce: base64(nonce), mac, aid}
```
Potem usunięcie z magazynu i wpis w archiwum. Unpair od strony managera przychodzi pushem
(`pairing.status == "DELETED"`) i kończy się tym samym usunięciem lokalnym.

## Push (v1) i zmiany w v2

Wiadomość FCM z polem `data.encedo` = **string JSON**:
- `{"event": {"pid": "<pid>", ...}}` — aplikacja ignoruje resztę i odpytuje `allbypid` (od razu i po 1,2 s),
- `{"pairing": {"pid": "<pid>", "status": "DELETED"}}` — usunięcie parowania.

Do tego `notification.title/body`, żeby system pokazał powiadomienie w tle; dźwięki własne (`blackberry`,
`crystal`, `msn`) w `res/`. Przykłady w `messages/` to ogólne payloady z dokumentacji pluginu firebasex,
nie format Encedo.

**[v2]**
- Nadawca: FCM HTTP v1, jedna wiadomość z sekcjami `android` i `apns`, `data.encedo` bez zmian; nowy format
  ustalony na podstawie obecnego JSON-a z backendu.
- Nowy endpoint `POST /notify/subscribers/token {pid, aid, fid, nonce, mac}`,
  `mac = HMAC-SHA256(bytes(nonce) || utf8(fid), X25519(aid_prv, epk))` po `POST /notify/session {aid}`.
  Aplikacja woła go po każdym `onTokenRefresh` i przy starcie, gdy token różni się od zapisanego.
- Normalizacja `pid`: v1 miesza `pid`, `pidA` (`Base64DecodeUrl`) i `pidx`; w v2 jedna funkcja
  `pid_to_path()` i jedna postać w magazynie (base64 standardowe, jak zwraca serwer).

## Tabela scope’ów (z `scopes.js`)

`system:config`, `system:upgrade`, `system:shutdown`, `storage:disk0`, `storage:disk0:rw`, `storage:disk1`,
`storage:disk1:rw`, `storage:disk` (lock), `logger:get`, `logger:del`, `keymgmt:use:(.*)#(.*)-(.*)`,
`keymgmt:use:(.*)#(.*)`, `keymgmt:use:(.*)`, `keymgmt:del`, `keymgmt:list`, `keymgmt:get`, `keymgmt:gen`,
`keymgmt:upd`, `keymgmt:imp`, `keymgmt:ecdh`, `keymgmt:derive`, `auth:ext:pair`.
Każdy wpis ma teksty pytania, sukcesu, odmowy i błędu oraz ekran docelowy; `_endpoints` w tym samym pliku
mapuje ścieżki HEM API na scope (dokumentacja, nie logika).

## Gdzie to jest w v1

| Fragment | Plik i linie |
|---|---|
| start, SecureKeyStore, SQLCipher, tabele | `www/js/index.js` 74–214 |
| rejestracja FCM, blokada, ustawienia | `www/js/index.js` 216–492, `www/js/base.js` 39–72 |
| dekrypcja scope, ekran decyzji, odpowiedź | `www/js/index.js` 656–960 |
| odpytanie `allbypid`, garbage collector | `www/js/index.js` 1006–1142 |
| obsługa pusha | `www/js/index.js` 1206–1241 |
| parowanie po QR | `www/js/index.js` 1252–1416, `www/js/base.js` 182–234 |
| unpair | `www/js/index.js` 1944–2030 |
| JWT, base64, HMAC helpers | `www/js/base.js` 163–284 |
| skaner QR (plugin) | `www/js/index.js` 1604–1722 |
