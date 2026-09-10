# Parytet z v1 — stan na 10 września 2026 (build dev 15)

Legenda: ✅ przeniesione i sprawdzone na żywo · 🟡 przeniesione, czeka na test na żywo · ⬜ nie przeniesione ·
➖ celowo pominięte (decyzja z PLAN-V2).

## Protokół i broker

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Parowanie przez QR (`GET link`, JWT, `POST link {reply, fid, mac, aid}`) | ✅ | Manager v1 i v2, żywy HEM, broker routuje zdarzenia do telefonu |
| „Already paired” po `eid` | ✅ | ekran z blokadą przycisku Pair |
| Odmowa parowania (`DELETE link`) | 🟡 | ścieżka rzadka, niesprawdzona na żywo |
| X25519, HMAC-SHA256, AES-128-CBC, JWT HS256 | ✅ | wektory z v1 bajt w bajt, `cargo test` |
| `allbypid` → `GET event` → dekrypcja scope | ✅ | żywy HEM + OIDC, build 15 |
| Weryfikacja MAC scope, `exp` przed pokazaniem | ✅ | **[v2]** v1 tego nie robiło |
| Tabela scope’ów i teksty | ✅ | `scope.rs`, nowy głos (STYLE.md), `keytype2string` |
| Allow z okresem 15 min / 1 h / 8 h / 24 h i `:rw` | ✅ | potwierdzone w OIDC |
| Deny (`DELETE event`) | ✅ | |
| Mapowanie 401 / 404 / 410 cancelled / 410 / 5xx | 🟡 | typowane błędy, ekran z komunikatem; ścieżki błędów niesprawdzone na żywo |
| Unpair z aplikacji (`/notify/session` + `subscribers/delete`) | 🟡 | broker odpowiada 404 także Managerowi, do wyjaśnienia po stronie brokera; jest „Remove from this phone anyway” |
| Unpair od strony Managera (push `pairing.status == DELETED`) | 🟡 | obsługa jest, czeka na push z backendu |
| Geolokalizacja wystawcy (`ipinfo_eid`) | ✅ | na ekranie parowania i żądania |
| Odświeżanie tokena push do brokera | 🟡 | **[v2]** klient gotowy, endpoint `/notify/subscribers/token` nie istnieje w backendzie |

## Push

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Rejestracja FCM, token, uprawnienie | ✅ | własny plugin `encedo-push` |
| Token wysyłany przy parowaniu (`fid`) | ✅ | |
| Odbiór pusha w tle i na wierzchu, tap w powiadomienie | ✅ | backend załatany `send_fcm_v2` (FCM HTTP v1), potwierdzone 10 września |
| Odpytanie po pushu, po powrocie z tła, po odblokowaniu | ✅ | plus co 15 s na ekranie Now (do usunięcia, gdy push będzie pewny) |
| Kanał powiadomień | ✅ | `encedo_requests`, tworzony przez aplikację, adresowany przez backend |
| Własne dźwięki (`blackberry`, `crystal`, `msn`) | ➖ | domyślny dźwięk systemu wystarcza |

## Magazyn i bezpieczeństwo

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Szyfrowany magazyn urządzeń, ustawień, archiwum | ✅ | AES-256-GCM, klucz w Android Keystore (v1: SQLCipher + SecureKeyStore) |
| Archiwum zdarzeń z filtrem po urządzeniu | ✅ | plus wyszukiwarka |
| Blokada biometryczna przy starcie i po powrocie z tła | ✅ | emulator z PIN-em i S24 |
| Blokada zrzutów ekranu (`FLAG_SECURE`) | ✅ | tylko pakiet produkcyjny |
| Klucze prywatne poza webview | ✅ | **[v2]** v1 trzymało wszystko w JS |
| Wykrycie i usunięcie danych v1 po aktualizacji | 🟡 | `legacy.rs`, test na plikach; prawdziwa aktualizacja z Play dopiero przy wydaniu |
| Blokada orientacji (portret) | ⬜ | jedna linia w manifeście, do zrobienia |

## Ekrany

| Ekran v1 | Stan | Uwagi |
|---|---|---|
| Welcome / onboarding | ✅ | jeden krok z ustawieniami blokady |
| Dashboard / lista urządzeń / szczegóły | ✅ | Now, Modules, moduł |
| Parowanie z kamerą | ✅ | zoom, wklejenie kodu, sprawdzenie `hash` |
| Żądanie z opcjami | ✅ | |
| Wynik: granted / denied / błąd | ✅ | plus expired, cancelled, paired, unpaired |
| Archiwum | ✅ | |
| Ustawienia | ✅ | biometria, tło, motyw, push, diagnostyka |
| O aplikacji | ✅ | numer buildu |
| Ukryta konsola (`consolelog`) | ✅ | zastąpiona kartą Diagnostics z Copy |
| Brak sieci / awaria | ✅ | |
| Shake-to-lock | ➖ | |
| FAQ, Terms, Help, Tutorial, Tips | ➖ | |
| Crashlytics, Analytics | ➖ | |

## Wydanie

| Element | Stan | Uwagi |
|---|---|---|
| Build Android z Vostro, numerowane APK dev | ✅ | `scripts/vostro-dev-apk.sh` |
| Podpis kluczem upload, AAB do Play | ⬜ | wstrzymane: nowa firma RKV zamiast Encedo, nowe konto Google, nowy projekt Firebase i nowe klucze (patrz „Przeprowadzka na RKV”) |
| iOS — na telefonie | ✅ | iPhone 15 Pro (iOS 26.6.1), podpis darmowym zespołem: parowanie, biometria, magazyn na Keychainie, skaner, odpytywanie brokera |
| iOS — push (kod) | 🟡 | plugin z Firebase Messaging działa w aplikacji; token APNs nie powstanie bez uprawnienia `aps-environment`, którego darmowy zespół nie daje (sprawdzone: `codesign -d --entitlements` pokazuje tylko `application-identifier`, `team-identifier`, `get-task-allow`) |
| iOS — podpis i wydanie | ⬜ | konto Apple Developer, profil z uprawnieniem push, TestFlight |
| Zdalne repo | ⬜ | push wstrzymany (konfigi Firebase w historii, repo publiczne) |

## Przeprowadzka na RKV (decyzja z 10 września 2026)

Encedo się zamyka, wydanie pójdzie z konta nowej firmy. Co to zmienia, zanim
powstanie pierwszy build produkcyjny:

| Rzecz | Skutek |
|---|---|
| Nowy projekt Firebase | nowy `google-services.json` → nowe `firebase.xml` (`scripts/firebase-res.py`), nowe konto serwisowe w backendzie (`$FCM_V2_SERVICE_ACCOUNT`) |
| Tokeny push z v1 | należą do starego projektu; po aktualizacji aplikacja rejestruje się w nowym i wysyła nowy token przy ponownym parowaniu. Dopóki stary projekt żyje, backend musi umieć wysłać z obu kont albo v1 przestaje dostawać pushe |
| Nowe konto Google Play | listing v1 (`com.encedo.mobile.auth.android`) jest na koncie Encedo. Albo przeniesienie aplikacji na nowe konto (Play Console → transfer, wymaga obu kont), albo nowy listing z nową nazwą pakietu i utratą ciągłości aktualizacji dla obecnych użytkowników |
| Klucz podpisu | przy transferze listingu klucz upload zostaje; przy nowym listingu powstaje nowy. `keystore.jks` z v1 ma sens tylko w pierwszym wariancie |
| Nazwa pakietu | zmiana (np. `pl.rkv.…`) oznacza nową aplikację w Play i nowy bundle id na iOS; do rozstrzygnięcia razem z decyzją o transferze |
