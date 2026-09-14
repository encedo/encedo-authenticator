# Parytet z v1 — stan na 12 września 2026 (build dev 15, dziennik nie był jeszcze na telefonie)

Legenda: ✅ przeniesione i sprawdzone na żywo · 🟡 przeniesione, czeka na test na żywo · ⬜ nie przeniesione ·
➖ celowo pominięte (decyzja z PLAN-V2) · 🧪 pokryte testem automatycznym.

Testy: `cargo test` w `src-tauri` uruchamia 24 testy rdzenia (`tests/core.rs`) przeciwko
brokerowi na 127.0.0.1 i modułowi, który sprawdza to, co telefon wysyła (`encedo-protocol`
z cechą `hem`). Symulacja nie zastępuje testu na żywym HEM, ale trzyma ścieżki błędów.

## Protokół i broker

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Parowanie przez QR (`GET link`, JWT, `POST link {reply, fid, mac, aid}`) | ✅ 🧪 | Manager v1 i v2, żywy HEM, broker routuje zdarzenia do telefonu |
| „Already paired” po `eid` | ✅ 🧪 | ekran z blokadą przycisku Pair |
| Odmowa parowania (`DELETE link`) | 🟡 🧪 | ścieżka rzadka, na żywo niesprawdzona; test sprawdza DELETE i wpis w archiwum |
| X25519, HMAC-SHA256, AES-128-CBC, JWT HS256 | ✅ | wektory z v1 bajt w bajt, `cargo test` |
| `allbypid` → `GET event` → dekrypcja scope | ✅ 🧪 | żywy HEM + OIDC, build 15; test: zdarzenie otwierane raz, `pid` w ścieżce w formie URL |
| Weryfikacja MAC scope, `exp` przed pokazaniem | ✅ 🧪 | **[v2]** v1 tego nie robiło; testy z podmienionym MAC-iem i z wygasłym zdarzeniem |
| Tabela scope’ów i teksty | ✅ | `scope.rs`, nowy głos (STYLE.md), `keytype2string` |
| Allow z okresem 15 min / 1 h / 8 h / 24 h i `:rw` | ✅ 🧪 | potwierdzone w OIDC; w teście moduł weryfikuje `authreply`, okres spoza listy spada do 15 min |
| Deny (`DELETE event`) | ✅ 🧪 | |
| Mapowanie 401 / 404 / 410 cancelled / 410 / 5xx | 🟡 🧪 | każdy kod (plus 418 i śmieci w odpowiedzi) sprawdzony w testach; na żywo niesprawdzone |
| Unpair z aplikacji (`/notify/session` + `subscribers/delete`) | 🟡 🧪 | broker odpowiada 404 także Managerowi, do wyjaśnienia po stronie brokera; test: dowód z nonce przyjęty przez moduł, 404 zostawia moduł, „Remove from this phone anyway” go usuwa |
| Unpair od strony Managera (push `pairing.status == DELETED`) | 🟡 🧪 | obsługa jest, czeka na push z backendu |
| Geolokalizacja wystawcy (`ipinfo_eid`) | ✅ | na ekranie parowania i żądania |
| Odświeżanie tokena push do brokera | 🟡 🧪 | **[v2]** klient gotowy, endpoint `/notify/subscribers/token` nie istnieje w backendzie; test sprawdza MAC i to, że 404 nie jest błędem |

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
| Szyfrowany magazyn urządzeń, ustawień, archiwum | ✅ | AES-256-GCM, klucz owijający w Android Keystore / Keychainie (v1: SQLCipher + SecureKeyStore) |
| Klucz owijający wiązany z człowiekiem | ✅ 🧪 | **[v2]** `setUserAuthenticationRequired` + okno 30 s (biometria albo PIN); na iOS `.userPresence`, czyli pytanie przy każdym użyciu. Root nie odszyfruje magazynu bez potwierdzenia |
| Klucz tylko przy odblokowanym ekranie, StrongBox | ✅ | **[v2]** `setUnlockedDeviceRequired(true)` i `setIsStrongBoxBacked(true)` z cichym fallbackiem |
| Kopie zapasowe i transfer na nowy telefon wyłączone | ✅ | **[v2]** `allowBackup=false`, `fullBackupContent=false`, `dataExtractionRules` wyklucza wszystko |
| Start od zera po utracie klucza | ✅ 🧪 | **[v2]** `key_lost` → ekran z ceną decyzji i `store_reset`; pierwsza linia dziennika mówi, co przepadło |
| Archiwum zdarzeń z filtrem po urządzeniu | ✅ 🧪 | zastąpione dziennikiem: rodziny (answers / modules / push / broker / app / trace), rozwijanie wpisu, sortowanie, filtr modułu, szukajka |
| Blokada biometryczna przy starcie i po powrocie z tła | ✅ | emulator z PIN-em i S24 |
| Blokada zrzutów ekranu (`FLAG_SECURE`) | ✅ | tylko pakiet produkcyjny |
| Klucze prywatne poza webview | ✅ | **[v2]** v1 trzymało wszystko w JS |
| Dziennik z pieczęcią (łańcuch SHA-256) | ✅ 🧪 | **[v2]** odpowiedzi, parowania i pushe pieczętują poprzedni wpis; `log_verify` wskazuje miejsce, gdzie łańcuch pęka |
| Retencja 90 dni | ✅ 🧪 | decyzja właściciela: wiek, nie liczba; sufit 20 000 / 5 000 wpisów tylko jako zabezpieczenie pliku |
| Wykrycie i usunięcie danych v1 po aktualizacji | 🟡 | `legacy.rs`, test na plikach; prawdziwa aktualizacja z Play dopiero przy wydaniu |
| Blokada orientacji (portret) | ✅ | manifest Androida i `Info.ios.plist`; portret na S24 i na iPhonie |

## Ekrany

| Ekran v1 | Stan | Uwagi |
|---|---|---|
| Welcome / onboarding | ✅ | jeden krok z ustawieniami blokady |
| Dashboard / lista urządzeń / szczegóły | ✅ | Now, Modules, moduł |
| Parowanie z kamerą | ✅ | zoom, wklejenie kodu, sprawdzenie `hash` |
| Żądanie z opcjami | ✅ | |
| Wynik: granted / denied / błąd | ✅ | plus expired, cancelled, paired, unpaired |
| Archiwum | ✅ | jako zakładka History: dziennik wszystkiego, co telefon zrobił |
| Ustawienia | ✅ | biometria, tło, motyw, push, broker, stan pieczęci; trzy karty diagnostyczne przeniesione do Historii |
| O aplikacji | ✅ | wersja, broker, protokół, magazyn, dane wydawcy i czego brakuje do sklepu |
| Lista bibliotek open source | ✅ | **[v2]** 285 pozycji z wersją, licencją i właścicielem praw, zbierane z builda przez `scripts/licences.py`; osobny ekran z szukajką, tekstami licencji i „Copy all” do listingu |
| Ukryta konsola (`consolelog`) | ✅ | zastąpiona rodziną Trace w Historii (Copy, Clear); karta Diagnostics zniknęła |
| Brak sieci / awaria | ✅ | |
| Shake-to-lock | ➖ | |
| FAQ, Terms, Help, Tutorial, Tips | ➖ | |
| Crashlytics, Analytics | ➖ | |

## Wydanie

| Element | Stan | Uwagi |
|---|---|---|
| Build Android z Vostro, numerowane APK dev | ✅ | `scripts/vostro-dev-apk.sh`, wysyłka przez `scripts/sync-vostro.sh` |
| Wymuszona aktualizacja po incydencie | ✅ 🧪 | **[v2]** Play In-App Updates: `inAppUpdatePriority` ≥ 4 blokuje starą wersję pełnoekranową planszą, 1–3 daje pasek na Now; werdykt zapamiętany, więc odcięcie sieci go nie omija. Na iOS brak odpowiednika — do zrobienia przez `itunes.apple.com/lookup` po publikacji |
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
