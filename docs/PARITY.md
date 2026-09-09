# Parytet z v1 — stan na 9 września 2026 (build dev 14)

Legenda: ✅ przeniesione i sprawdzone na żywo · 🟡 przeniesione, czeka na test na żywo · ⬜ nie przeniesione ·
➖ celowo pominięte (decyzja z PLAN-V2).

## Protokół i broker

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Parowanie przez QR (`GET link`, JWT, `POST link {reply, fid, mac, aid}`) | ✅ | Manager v1 i v2, żywy HEM, broker routuje zdarzenia do telefonu |
| „Already paired” po `eid` | 🟡 | ekran z blokadą przycisku Pair |
| Odmowa parowania (`DELETE link`) | 🟡 | |
| X25519, HMAC-SHA256, AES-128-CBC, JWT HS256 | ✅ | wektory z v1 bajt w bajt, `cargo test` |
| `allbypid` → `GET event` → dekrypcja scope | 🟡 | routing potwierdzony (build 13), odczyt zdarzenia od buildu 14 |
| Weryfikacja MAC scope, `exp` przed pokazaniem | 🟡 | **[v2]** v1 tego nie robiło |
| Tabela scope’ów i teksty | ✅ | `scope.rs`, nowy głos (STYLE.md), `keytype2string` |
| Allow z okresem 15 min / 1 h / 8 h / 24 h i `:rw` | 🟡 | |
| Deny (`DELETE event`) | 🟡 | |
| Mapowanie 401 / 404 / 410 cancelled / 410 / 5xx | 🟡 | typowane błędy, ekran z komunikatem |
| Unpair z aplikacji (`/notify/session` + `subscribers/delete`) | 🟡 | broker odpowiada 404 także Managerowi, do wyjaśnienia po stronie brokera; jest „Remove from this phone anyway” |
| Unpair od strony Managera (push `pairing.status == DELETED`) | 🟡 | obsługa jest, czeka na push z backendu |
| Geolokalizacja wystawcy (`ipinfo_eid`) | ✅ | na ekranie parowania i żądania |
| Odświeżanie tokena push do brokera | 🟡 | **[v2]** klient gotowy, endpoint `/notify/subscribers/token` nie istnieje w backendzie |

## Push

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Rejestracja FCM, token, uprawnienie | ✅ | własny plugin `encedo-push` |
| Token wysyłany przy parowaniu (`fid`) | ✅ | |
| Odbiór pusha w tle i na wierzchu, tap w powiadomienie | 🟡 | klient gotowy, backend wysyła legacy API (wyłączone), czeka na nadawcę FCM v1 |
| Odpytanie po pushu, po powrocie z tła, po odblokowaniu | 🟡 | plus co 15 s na ekranie Now do czasu pusha |
| Kanał powiadomień i dźwięki (`blackberry`, `crystal`, `msn`) | ⬜ | razem z nadawcą FCM v1 |

## Magazyn i bezpieczeństwo

| Funkcja v1 | Stan | Uwagi |
|---|---|---|
| Szyfrowany magazyn urządzeń, ustawień, archiwum | ✅ | AES-256-GCM, klucz w Android Keystore (v1: SQLCipher + SecureKeyStore) |
| Archiwum zdarzeń z filtrem po urządzeniu | ✅ | plus wyszukiwarka |
| Blokada biometryczna przy starcie i po powrocie z tła | 🟡 | emulator z PIN-em ✅, S24 do potwierdzenia |
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
| Żądanie z opcjami | 🟡 | |
| Wynik: granted / denied / błąd | 🟡 | plus expired, cancelled, paired, unpaired |
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
| Podpis kluczem upload, AAB do Play | 🟡 | Gradle i skrypt gotowe, sekrety do skopiowania na Vostro, Play App Signing do sprawdzenia |
| iOS | ⬜ | `tauri ios init` na Mac Mini, własny plugin push po stronie iOS, konto Apple Developer |
| Zdalne repo | ⬜ | push wstrzymany (konfigi Firebase w historii, repo publiczne) |
