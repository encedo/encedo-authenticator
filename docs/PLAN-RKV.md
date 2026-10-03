# Plan wydania v2 pod RKV

Stan: 10 września 2026. Aplikacja na Androidzie ma parytet z v1 (`docs/PARITY.md`),
push działa przez załatany backend. Encedo się zamyka, wydanie idzie z konta nowej
firmy. Ten dokument opisuje drogę od działającego builda dev do dwóch sklepów.

## Zasada

Zmiana firmy dotyka czterech tożsamości i każda ciągnie za sobą inne skutki:
nazwa pakietu (Play, App Store), projekt Firebase (push), klucz podpisu (ciągłość
aktualizacji) i marka w tekstach. Wszystkie decyzje z fazy A muszą zapaść razem,
bo każda osobno cofa pracę w kolejnych fazach.

## Faza A — decyzje i konta (właściciel, blokuje resztę)

| # | Decyzja | Warianty | Skutek |
|---|---|---|---|
| A1 | Listing w Google Play | **zdecydowane 13 września 2026: nowy listing**. RKV jest właścicielem Encedo, więc marka zostaje, ale v2 wchodzi jako osobna pozycja | v1 żyje obok do wygaszenia; użytkownicy v1 **nie** dostaną v2 automatycznie, muszą zainstalować ją sami; potrzebny **nowy klucz upload** (ten z v1 dotyczy tamtego listingu) |
| A2 | Nazwa pakietu | **zdecydowane: `com.encedo.mobile.auth.android.v2`**, na iOS `com.encedo.mobile.auth.ios.v2` | obie wersje mogą stać na jednym telefonie; na Androidzie wystarczyła zmiana `applicationId` (przestrzeń nazw Kotlina zostaje, więc nic nie trzeba regenerować), na iOS `project.yml` i projekt Xcode |
| A3 | Nazwa produktu | **zdecydowane 13 września 2026: „Encedo HEM Authenticator”** | w aplikacji, w `tauri.conf.json`, w `strings.xml`, w `Info.ios.plist` i w notce licencyjnej; wydawcą jest RKV, nazwa produktu została przy Encedo HEM |
| A4 | Konto Apple Developer | organizacja (wymaga numeru D-U-N-S, kilka dni) albo indywidualne | 99 USD/rok; organizacja wygląda poważniej w App Store i pozwala na więcej ról |
| A5 | Projekt Firebase | nowy projekt na koncie RKV | nowe `google-services.json`, nowe konto serwisowe w backendzie, nowy `GoogleService-Info.plist` dla iOS |
| A6 | Los starego projektu Firebase | trzymać N miesięcy czy zamknąć | telefony z v1 dostają push tylko dopóki stary projekt żyje i backend umie wysyłać z obu kont |
| A7 | Polityka prywatności i regulamin | adres na rkv.pl | oba sklepy wymagają publicznego adresu przed publikacją |

Bez A1–A3 nie ma sensu robić buildów produkcyjnych; bez A4 nie ma iOS.

## Faza B — rebranding w kodzie (1–2 dni, po A1–A3)

- Identyfikatory: `tauri.conf.json`, `tauri.android.conf.json`, `tauri.ios.conf.json`.
  Zmiana nazwy pakietu wymaga skasowania i ponownego `tauri android init` / `ios init`,
  bo Tauri trzyma nazwę w ścieżce katalogów Kotlina.
- Teksty: nazwa produktu w mastheadzie, ekranie About, `productName`, nazwa w launcherze.
- Ikony: zrobione (głowa z v1 w zieleni rkv.pl, launcher zwykły i adaptacyjny, monochrom,
  ikona powiadomienia). Do sprawdzenia po zmianie nazwy: nic.
- Firebase: `scripts/firebase-res.py` z nowym `google-services.json`; ten sam skrypt
  omija plugin Gradle, więc warianty `.dev` dalej się budują.
- Stopka: „© Encedo” → nowa firma, adres polityki prywatności.

## Faza C — Android w Google Play (2–3 dni pracy, plus czas recenzji)

1. Sekrety na Vostro: `keystore.jks` i `keystore.properties` w `~/secrets/encedo-authenticator/`
   (przy wariancie A1a) albo nowy keystore (A1b). Sprawdzić w Play Console, czy konto ma
   Play App Signing i czy `keystore.jks` to klucz upload.
2. AAB z GitHub Actions (`.github/workflows/android.yml`, każdy push na `main` i ręcznie):
   bez sekretów keystore wychodzi niepodpisany AAB i APK z sumami SHA-256, podpis offline
   kluczem upload (`jarsigner` dla AAB, `apksigner` dla APK) i dopiero wtedy do Play; z sekretami
   `UPLOAD_KEYSTORE_B64`, `UPLOAD_KEYSTORE_PASSWORD`, `UPLOAD_KEY_ALIAS`, `UPLOAD_KEY_PASSWORD`
   Gradle podpisuje na miejscu. Push wymaga sekretu `GOOGLE_SERVICES_JSON`. `versionCode`
   z `tauri.conf.json` (2000000+, wyżej niż 100030 z v1) albo z pola przy ręcznym uruchomieniu.
   Zapas lokalny: `scripts/vostro-release-aab.sh` na vostro.
3. Play Console: opis, zrzuty (telefon, 8 sztuk z buildów dev), ikona 512×512,
   grafika promocyjna 1024×500, ocena treści, formularz Data safety (odpowiedzi:
   dane pozostają na urządzeniu, wysyłany jest tylko podpisany werdykt i token push),
   polityka prywatności.
4. Ścieżka: internal testing (właściciel + córka) → closed → produkcja.
4a. **Wydanie po incydencie bezpieczeństwa.** Aplikacja ma blokadę starych wersji
   (`plugins/encedo-update`, ekran „This version cannot be used”), ale wisi ona na
   jednym polu przy publikacji: `inAppUpdatePriority` wydania. Zasada: 4 albo 5 dla
   wydania, którego nikt nie może pominąć, 1–3 dla zwykłej poprawki, 0 gdy nie ma
   po co zawracać głowy. To pole ustawia się przez Publishing API (`edits.tracks.update`),
   nie widziałem go w konsoli — do sprawdzenia przy pierwszym wydaniu. **Opis, co było
   nie tak, piszemy w notatkach wydania**, bo API Play nie przenosi żadnego tekstu,
   a plansza w aplikacji odsyła właśnie tam.
5. v2 to osobny pakiet, więc **nie widzi danych v1** (inny katalog aplikacji) i
   `legacy.rs` nic tam nie sprząta — v1 zostaje na telefonie, dopóki właściciel jej
   nie odinstaluje. W opisie w Play warto napisać wprost: zainstaluj v2, sparuj
   moduły na nowo, a v1 usuń, gdy wszystko działa. Do rozważenia przy wygaszaniu
   v1: aktualizacja v1, która mówi, gdzie jest następca.
6. W projekcie Firebase (A5) potrzebne wpisy aplikacji dla nowych identyfikatorów
   `…android.v2` i `…ios.v2`; `scripts/firebase-res.py` omija plugin Gradle, więc
   build zadziała nawet przed ich dodaniem, ale push pójdzie dopiero z nimi.

## Faza D — iOS (5–8 dni po A4)

Stan: `tauri ios init` przeszedł na Mac Mini, projekt Xcode wygenerowany, build na
symulator w toku. Brakuje trzech rzeczy, żeby aplikacja była kompletna:

1. **Magazyn**: klucz danych na iOS leży dziś w pliku (`DevFileSecret`). Do zrobienia:
   iOS-owa połowa pluginu `encedo-keystore` na Keychainie z `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`.
2. **Push**: napisane (`plugins/encedo-push/ios`, Firebase Messaging przez SPM). v1 na iOS
   chodziło przez FCM z tym samym nadawcą, a projekt `encedo-mobile-authenticator` ma
   aplikację iOS o bundle id `com.encedo.mobile.auth.ios` (GOOGLE_APP_ID `1:720382161304:ios:1a2a…`)
   i klucz APNs. Zostaje: `GoogleService-Info.plist` z nowego projektu w `gen/apple/assets/`
   (poza repo), profil z uprawnieniem `aps-environment` i test na telefonie —
   symulator nie wystawia tokena APNs, więc dalej się nie da sprawdzić.
3. **Podpis**: konto z A4 w Xcode, profil provisioning, TestFlight. Darmowy Apple ID
   wystarczy do wszystkiego poza pushem: sprawdzone 10 września na iPhonie 15 Pro,
   działa parowanie, biometria, skaner i odpytywanie brokera, a `aps-environment`
   nie da się podpisać darmowym zespołem, więc APNs nie wystawia tokena.
   Push na iOS = płatny program, bez wyjątków.

Reszta działa bez zmian: skaner (plugin ma część iOS), biometria (Face ID przez ten sam
plugin), protokół i magazyn w Ruście, UI. `Info.ios.plist` ma już powody dostępu do
kamery i Face ID, portret i tryb `remote-notification`.

### Push na iOS: co odblokowuje płatne konto

Kod jest gotowy i sprawdzony, zostaje wyłącznie strona kont i podpisu:

1. Apple Developer Program na koncie RKV (99 USD/rok). Konto firmowe wymaga numeru
   D-U-N-S i kilku dni; indywidualne jest od ręki.
2. W portalu Apple: App ID dla bundle id aplikacji z włączoną zdolnością **Push Notifications**.
3. Klucz APNs `.p8` w portalu Apple → wgrany w konsoli Firebase (Project settings →
   Cloud Messaging → APNs Authentication Key, razem z Key ID i Team ID). Klucz z czasów v1
   należy do zespołu Encedo, więc pod nowym zespołem potrzebny jest nowy.
4. Build z `aps-environment` (jest już w `gen/apple/…entitlements`; skrypt dev go usuwa,
   bo darmowy zespół go nie podpisze — ścieżka produkcyjna go zachowuje).
5. Test na telefonie: token FCM w Ustawieniach, `send_fcm_v2` z backendu, powiadomienie
   przy zamkniętej aplikacji i tap prowadzący do ekranu żądania.
6. TestFlight zamiast kabla, gdy trzeba dać aplikację komuś spoza biurka.

**App Review**: recenzent musi umieć sparować telefon. Bez dostępu do HEM utknie.
Do przygotowania: konto testowe i osiągalny HEM z kodem QR w notatkach dla recenzenta,
albo tryb demonstracyjny w aplikacji. Do rozstrzygnięcia przed pierwszym zgłoszeniem.

## Faza E — backend (równolegle, właściciel)

- `send_fcm_v2` z `tools/php-fcm-v1/` działa jako most; docelowo nadawca w Node.
- Do wyjaśnienia: 404 z `/notify/subscribers/delete` (dotyczy Managera i telefonu,
  więc unpair działa dziś tylko lokalnie).
- Nowy endpoint `POST /notify/subscribers/token` (odświeżanie tokena push);
  klient w aplikacji gotowy i toleruje jego brak.
- Przy zmianie projektu Firebase: nowe konto serwisowe w nadawcy, ewentualnie
  wysyłka z dwóch projektów przez czas życia v1.

## Faza F — utwardzenie i domknięcie (2–3 dni)

- CSP bez `unsafe-eval`, przegląd `capabilities`, logi bez sekretów (dziennik
  diagnostyczny już nie zapisuje materiału kluczy).
- Zdalne repo: `github.com/encedo/encedo-authenticator`, publiczne, od 3 października
  2026. Historia przepisana bez konfigów Firebase; dziennik i opis maszyn poza repo.
- Usunąć odpytywanie co 15 s z ekranu Now, gdy push okaże się pewny.
- Wersjonowanie z jednego źródła, notatki wydania.

## Kolejność i czas

```
A (właściciel, dni–tygodnie)
├── B (1–2 dni) ──> C (2–3 dni + recenzja Play)
├── D (5–8 dni, wymaga A4) ──> TestFlight ──> App Store
└── E (backend, równolegle)
                    F (2–3 dni, przed publikacją)
```

Po stronie kodu to około dwóch tygodni pracy; kalendarz wyznaczą konta i recenzje
sklepów. Android może wyjść pierwszy, iOS zaraz po nim.
