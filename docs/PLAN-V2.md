# Encedo Mobile Authenticator v2 — plan działania

Stan: v0.2, 2 września 2026. Branch `v2`. Wersja HTML tego planu (te same treści, tokeny rkv.pl):
https://claude.ai/code/artifact/f4ba3fed-e466-4009-b3a8-fee980c5c756

Z Cordovy do Tauri 2. Ten sam protokół, nowa aplikacja, Android i iOS. Backend działa i zostaje bez
refaktoru; dotykamy go tylko tam, gdzie wysyła push i przyjmuje token urządzenia.

## Ustalone

| Temat | Ustalenie |
|---|---|
| Zakres | Tylko aplikacja mobilna. `api.encedo.com/notify` działa i zostaje w obecnym kształcie; refaktor backendu później. Flow z v1 testujemy od razu na żywym API. |
| Podejście | Przepisanie od zera („o 180°”). Z v1 zostaje protokół, model danych i zestaw ekranów; kod nie. |
| Push | FCM na obu platformach, jak w v1: z punktu widzenia backendu jeden nadawca i jeden typ tokena, APNs obsługuje Firebase. Stary nadawca korzystał z wyłączonego legacy API; przechodzimy na FCM HTTP v1 i porządkujemy format JSON, wychodząc od obecnego. |
| Platformy | Android jako aktualizacja istniejącej aplikacji w Google Play (`com.encedo.mobile.auth.android`, ostatnia publikacja 2 listopada 2025). iOS jako nowa publikacja (`com.encedo.mobile.auth.ios`, bundle jest już w projekcie Firebase `encedo-mobile-authenticator`). |
| Do zachowania | Push, szyfrowany magazyn lokalny, parowanie przez QR z kamery, blokada biometryczna przy starcie i po powrocie z tła, archiwum zdarzeń, odświeżanie tokena push (nowe). |
| Pomijamy na start | Blokada przy potrząśnięciu, blokada zrzutów ekranu na iOS, strony FAQ / Terms / Help (w v1 lorem ipsum), Crashlytics i Analytics. Blokadę zrzutów na Androidzie zostawiamy (jedna linia w `MainActivity`). |
| Styl | Tokeny kolorów, typografia i rytm sekcji z rkv.pl. Tryb jasny i ciemny od pierwszego ekranu. |

## Architektura

Trzy warstwy i jedna zasada: **klucze prywatne nie opuszczają Rusta**.

1. **Webview (Svelte 5 + TypeScript).** Dostaje wyłącznie dane do wyświetlenia (nazwa urządzenia, scope,
   wystawca, status) i wysyła decyzje (allow / deny, okres dostępu, zapis rw). Ekran to stan w store, bez
   routera URL.
2. **Rdzeń Rust (`src-tauri`).** Moduł protokołu (X25519, HMAC-SHA256, AES-128-CBC, JWT HS256), klient HTTP
   do notify API z mapowaniem 401 / 404 / 410 / 5xx, magazyn (plik szyfrowany AES-256-GCM, klucz w
   keychain / keystore), archiwum, maszyna stanów sesji (zablokowana, odblokowana, w trakcie żądania).
3. **Pluginy natywne.** `barcode-scanner` i `biometric` (oficjalne), `keyring` dla klucza magazynu, `os` dla
   nazwy urządzenia jako label parowania, push za własnym interfejsem `PushProvider`.

Dlaczego tak: w v1 klucz prywatny parowania i cały protokół żyły w JavaScripcie w webview obok CSP
`default-src *`. W v2 webview nie widzi kluczy, a moduł protokołu ma testy z wektorami wygenerowanymi z v1.
Ten sam frontend uruchamiamy jako aplikację desktopową z zamockowanymi pluginami, więc większość pracy nad
UI idzie bez emulatora.

## Decyzje

| # | Temat | Status | Wybór |
|---|---|---|---|
| 1 | Transport push | ustalone | FCM na Androidzie i iOS. Aplikacja: `tauri-plugin-fcm` (srod; `getToken`, `onTokenRefresh`, na iOS wymiana APNs → FCM przez Firebase SDK; wersja 0.1, więc za interfejsem `PushProvider`). Plan B: własny plugin Kotlin + Swift, potrzebny tylko token i jego odświeżenia. Backend: FCM HTTP v1 (konto serwisowe, OAuth2), jedna wiadomość z sekcjami `notification`, `data.encedo`, `android`, `apns`. Push tylko budzi aplikację; ona odpytuje `allbypid`. |
| 2 | Magazyn lokalny | ustalone | Plik JSON szyfrowany AES-256-GCM (ustawienia + urządzenia) i drugi append-only z archiwum. Klucz 32 B w natywnym keystore, zapis atomowy, format z numerem wersji. Bez SQLite: oficjalny `tauri-plugin-sql` nie szyfruje, fork z SQLCipher ma mobile „untested”, kompilacja OpenSSL pod NDK/iOS to koszt bez zysku. |
| 3 | Klucz w keychain / keystore | ustalone | Natywnie: Android Keystore + iOS Keychain przez `tauri-plugin-keyring` za interfejsem `SecretStore` (młody projekt, plan B to własny plugin, ~150 linii na platformę). Odrzucone `tauri-plugin-keystore` (impierce) i `tauri-plugin-biometry`: wymuszają biometrię przy każdym odczycie. Blokada biometryczna osobno, przez oficjalny `tauri-plugin-biometric`. |
| 4 | Framework UI | otwarte, domyślnie Svelte | Svelte 5 + TypeScript + Vite (szablon `create-tauri-app`). Alternatywa Vue 3; vanilla TS też przejdzie, ale drożej w utrzymaniu. |
| 5 | Dane z v1 | otwarte, domyślnie brak migracji | Po aktualizacji na Androidzie zostają baza SQLCipher i preferencje pluginu `secure-key-store`; v2 je wykrywa, usuwa i prosi o ponowne sparowanie (jeden skan QR na urządzenie). Odczyt starej bazy to kilka dni pracy dla jednorazowego efektu. |
| 6 | Identyfikatory i podpis | do sprawdzenia | Android: ten sam package i ten sam klucz upload (`keystore.jks`, poza repo), sprawdzić Play App Signing. iOS: Apple Developer Program, klucz APNs (.p8) w konsoli Firebase, TestFlight; build wymaga macOS z Xcode. Wersja z jednego źródła (`tauri.conf.json`); v1 ma trzy różne numery. |

## Fazy

Szacunki zgrubne, jedna osoba, dni robocze. Backend działa, więc fazy 2–5 testujemy na żywym API;
migracja nadawcy push po stronie backendu może iść równolegle z 4 i 5.

### Faza 0 — Przygotowanie i wektory testowe (1–2 dni)
- Dostępy: projekt Firebase, Play Console, Apple Developer. Środowisko: Rust + cele Android, Android SDK/NDK, JDK 17/21, Xcode na macOS.
- Wektory testowe (known-answer tests) wygenerowane z v1: `www/js/axlsign.js` i `www/js/crypto-js.js` działają w Node bez Cordovy. Parowanie, dekrypcja scope, odpowiedź, unpair: wejścia i oczekiwane wyjścia jako JSON.
- Szczęśliwą ścieżkę testujemy na żywym API od pierwszego dnia; mock notify tylko do ścieżek błędów (401 / 404 / 410) i pracy offline.
- Obecny format JSON pusha z backendu jako punkt wyjścia do nowego formatu FCM HTTP v1.
- Wynik: branch `v2`, plik z wektorami, mock błędów, nowy format pusha, checklista dostępów.

### Faza 1 — Szkielet Tauri (2–3 dni)
- `create-tauri-app` (Svelte + TS), `tauri android init`, `tauri ios init`, `capabilities/mobile.json` z minimalnymi uprawnieniami.
- Tokeny CSS z rkv.pl, oba motywy, safe-area, blokada orientacji w konfiguracji platform.
- CI: build Android AAB na Linuksie, iOS na runnerze macOS. Zastępuje Dockera `beevelop/ionic` i skrypty `.bat`.
- Wynik: pusta aplikacja na emulatorze, symulatorze i jako okno desktopowe.

### Faza 2 — Rdzeń protokołu w Ruście (4–6 dni)
- Crypto: `x25519-dalek`, `hmac` + `sha2`, `aes` + `cbc` (PKCS7), `base64`, JWT HS256 ręcznie z nagłówkiem `{ecdh, alg, typ}` w kolejności z v1.
- Przepływy: parowanie, odbiór i odpowiedź na żądanie, unpair (patrz `docs/PROTOCOL.md`). Klient HTTP na `reqwest` + rustls, timeouty, typowane błędy.
- Poprawki względem v1: MAC odszyfrowanego scope weryfikowany, `exp` sprawdzany przed pokazaniem ekranu, porównania w stałym czasie, `zeroize` na kluczach.
- Komendy Tauri wystawione do webview zwracają tylko dane prezentacyjne.
- Wynik: `cargo test` zielony na wektorach z v1; parowanie i żądanie przechodzą na żywym API przez odpytanie, jeszcze bez pusha.

### Faza 3 — Magazyn i sekrety (3–4 dni)
- `SecretStore` nad pluginem keyring; `EncryptedStore` z AES-256-GCM, zapisem atomowym i wersją formatu.
- Model: ustawienia; urządzenia (pid, eid, aid, aid_prv, label, host, email, iat); archiwum (nagłówek, opis, status, data, pid).
- Wykrywanie plików v1 przy pierwszym uruchomieniu, komunikat, czyszczenie.
- Wynik: parowanie zapisuje urządzenie, restart aplikacji je widzi.

### Faza 4 — Interfejs (6–8 dni)
- Design system: eyebrow + nagłówek, lista „ledger”, przyciski, przełącznik, zakładki, stany pusty / ładowanie / błąd.
- Ekrany: start i onboarding (jeden krok z ustawieniami), główny, urządzenia, szczegóły urządzenia, parowanie z podglądem kamery, żądanie dostępu z opcjami (15 min / 1 h / 8 h / 24 h, zapis rw), wynik (granted / denied / błąd), archiwum z filtrem po urządzeniu i wyszukiwarką, ustawienia, o aplikacji, blokada, awaria.
- Wynik: pełny przebieg na desktopie i emulatorze.

### Faza 5 — Integracje natywne (5–8 dni)
- Skaner QR: `barcode-scanner` w trybie `windowed`, przezroczysty webview, ramka i przycisk powrotu.
- Biometria: blokada przy starcie (opcjonalna) i po powrocie z tła; `allowDeviceCredential` jako fallback na PIN.
- Push: uprawnienie, token FCM, wysyłka tokena na serwer przy parowaniu i po każdej zmianie; odbiór w foreground i tap w powiadomienie prowadzą do odpytania `allbypid`.
- Cykl życia: pause → blokada, resume → odpytanie. Android: `FLAG_SECURE`.
- Wynik: prawdziwy push budzi aplikację na Androidzie i iOS.

### Faza 6 — Backend i testy end-to-end (3–5 dni)
- Backend: nadawca FCM HTTP v1 z nowym formatem JSON, endpoint aktualizacji tokena.
- Scenariusze z prawdziwym HEM: parowanie, każdy scope z tabeli v1, unpair z obu stron, wygaśnięcie, anulowanie, „obsłużone przez inną aplikację”, brak sieci, token po reinstalacji.
- Wynik: lista scenariuszy odhaczona na obu platformach.

### Faza 7 — Utwardzenie i wydanie (4–6 dni)
- CSP ścisłe, bez `unsafe-eval`, przegląd uprawnień capabilities, logi bez sekretów, eksport logu diagnostycznego zamiast ukrytej konsoli.
- Sekrety poza repo, jedno źródło wersji, polityka prywatności.
- Play internal testing i TestFlight, potem produkcja.
- Wynik: build w obu sklepach.

Razem: 28–42 dni roboczych. Największa wariancja: token FCM na iOS i środowisko Apple.

## Backend: co zostaje, co się zmienia

| Endpoint | Status | Uwagi |
|---|---|---|
| `GET / POST / DELETE <link z QR>` | zostaje | Parowanie: pobranie żądania, odpowiedź `{reply, fid, mac, aid}`, odmowa. |
| `POST /notify/event/data/allbypid` | zostaje | Odpytanie po pushu, po tapie w powiadomienie i po każdym powrocie z tła. Body `{pid: [...]}`. |
| `GET / POST / DELETE /notify/event/data/{event}/{pidx}` | zostaje | Pobranie, zgoda `{authreply, mac}`, odmowa. Kody 401 / 404 / 410 (`reason: cancelled`) / 5xx jak w v1. |
| `POST /notify/session` | zostaje | `{aid}` → `{epk}`. Klucz sesji do MAC-ów poniżej. |
| `POST /notify/subscribers/delete` | zostaje | `{pid, epk, nonce, mac, aid}`, `mac = HMAC(nonce, ECDH(aid_prv, epk))`. |
| nadawca push | zmiana | Legacy FCM → FCM HTTP v1 (konto serwisowe, OAuth2). Jedna wiadomość dla obu platform z sekcjami `android` i `apns`. Dane bez zmian: `data.encedo` = JSON `{event: {pid, ...}}` lub `{pairing: {pid, status}}`, plus tytuł i treść, żeby system pokazał powiadomienie w tle. Android: stały kanał, np. `encedo_requests`. Nowy format ustalamy na bazie obecnego. |
| `POST /notify/subscribers/token` | nowy | `{pid, aid, fid, nonce, mac}`, `mac = HMAC(nonce ‖ fid, ECDH(aid_prv, epk))` po `/notify/session`. Ten sam wzorzec co delete. v1 wysyła token tylko raz, w MAC-u parowania; po rotacji tokena serwer wysyła w próżnię. |
| pole `platform` w parowaniu | opcja | Przy jednym typie tokena niepotrzebne do routingu; może zostać do diagnostyki. |

## Styl: tokeny z rkv.pl

Od 7 września 2026 źródłem stylu jest `~/develop/encedo-manager/design/STYLE.md` (ten sam zestaw tokenów
plus głos: nagłówki jako zdania o stanie, mono dla wartości literalnych, „module” zamiast „device”, brak
wykrzykników i emoji, sekcja „For the HEM Authenticator specifically”). Tabela poniżej zostaje jako skrót.

Dwa akcenty z gotowym znaczeniem: **sealed** (zieleń) = zweryfikowane, sparowane, Allow; **exposed**
(rdza) = błąd MAC, wygasłe, odmowa, Deny. Żadnych innych akcentów.

| Token | Jasny | Ciemny |
|---|---|---|
| ground | `#F1F4F3` | `#0A0E0E` |
| surface | `#FFFFFF` | `#121918` |
| sunken | `#E7EBE9` | `#060909` |
| ink | `#0F1414` | `#E8F0ED` |
| ink-soft | `#3A4746` | `#AFBDB8` |
| muted | `#6A7773` | `#7C8C87` |
| rule / rule-firm | `#D5DCD9` / `#A9B5B1` | `#232D2C` / `#3A4644` |
| sealed / soft / line | `#0F5F4B` / `#E0EFE9` / `#96C6B5` | `#47C6A3` / `#102E27` / `#2C6858` |
| exposed / soft | `#94422A` / `#F5E6E0` | `#DE8467` / `#2C1B15` |
| band bg / ink / soft / rule | `#0E5544` / `#F2F8F5` / `#AFD2C5` / `#2E7A64` | `#0D2721` / `#E6F3ED` / `#9CBFB4` / `#2A5A4C` |

- Typografia: systemowy sans w tekście (`system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`), mono
  (`ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`) dla eyebrow, KID, PID, adresów i statusów,
  serif (`Georgia, ui-serif`) tylko w lede na ekranach informacyjnych. Bez pobierania fontów.
- Nagłówek ekranu: eyebrow mono 11.5 px, uppercase, letter-spacing .17em, kolor sealed; nagłówek 600,
  letter-spacing ujemny. Listy jako „ledger”: wiersze z górną linią 1–2 px, bez kart z cieniem.
- Przyciski: promień 5 px, mono 13 px, obrys 1 px. Primary na sealed-soft z obrysem sealed-line, plain z
  obrysem rule-firm, destrukcyjne na exposed. Wysokość dotykowa minimum 48 px, akcje w zasięgu kciuka.
- Ruch: przejścia ekranów 200 ms, zero animacji dekoracyjnych, `prefers-reduced-motion`.

## Ryzyka

| Poziom | Ryzyko | Plan |
|---|---|---|
| wysokie | Token FCM na iOS wymaga Firebase SDK w pluginie Tauri; `tauri-plugin-fcm` jest w wersji 0.1 | Własny plugin Swift + Kotlin, 2–3 dni. Od pluginu potrzebujemy tokena i odświeżeń, nic więcej. |
| wysokie | Środowisko Apple: macOS z Xcode, konto Apple Developer | Bez tego iOS na koniec, Android wychodzi pierwszy. |
| średnie | Push zadziała dopiero po migracji nadawcy na FCM v1 | Parowanie i żądania testujemy na żywym API przez odpytanie; żądanie pokazuje się po powrocie z tła albo ręcznym odświeżeniu. |
| średnie | Klucz podpisu: aktualizacja w Play wymaga tego samego klucza upload | Potwierdzić przed fazą 7, sprawdzić Play App Signing. |
| niskie | Plugin keyring młody | Za interfejsem, plan B na jedną fazę. |
| niskie | Cięcie z 30 „stron” v1 do ok. 10 ekranów | Jeśli FAQ / Terms / Help mają wrócić, +1–2 dni i treść po stronie Encedo. |

## Źródła
- v1: `config.xml`, `package.json`, `www/js/index.js`, `www/js/base.js`, `www/js/scopes.js`, `www/js/template.js`, `www/index.html`, `www/css/index.css`
- https://rkv.pl (tokeny CSS, typografia, sekcje)
- https://docs.encedo.com/hem-api/reference/api-reference/authorization/external-authenticator/registration.md
- https://docs.encedo.com/hem-api/reference/api-reference/authorization/external-authenticator/authentication.md
- https://v2.tauri.app/plugin/barcode-scanner/ · https://v2.tauri.app/plugin/biometric/
- https://github.com/srod/tauri-plugin-fcm · https://github.com/Choochmeque/tauri-plugin-notifications · https://github.com/yanqianglu/tauri-plugin-mobile-push
- https://github.com/charlesportwoodii/tauri-plugin-keyring · https://github.com/impierce/tauri-plugin-keystore · https://github.com/Choochmeque/tauri-plugin-biometry
- https://docs.rs/crate/tauri-plugin-rusqlite2/latest · https://github.com/tauri-apps/plugins-workspace/issues/7
