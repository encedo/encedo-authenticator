# Encedo Mobile Authenticator

Aplikacja mobilna do zatwierdzania żądań dostępu z Encedo HEM (push → allow/deny). Repo ma dwie gałęzie:

- `main`: snapshot v1 (Cordova 12, app 1.0.3 w Google Play jako `com.encedo.mobile.auth.android`). Tylko referencja protokołu i ekranów; nie rozwijamy.
- `v2`: przepisanie od zera w Tauri 2 na Android i iOS. Tu trwa praca.

Zacznij od `docs/`:

- `docs/PLAN-V2.md` — plan, decyzje (co ustalone, co domyślne), fazy 0–7, zmiany po stronie backendu, tokeny stylu.
- `docs/PROTOCOL.md` — bajtowy opis parowania, żądania i unpair z v1, z odsyłaczami do linii; podstawa modułu w Ruście.
- `../encedo-authenticator-private/ENVIRONMENT.md` — maszyny i jak na nich budować (poza repo, nie publikujemy).
- `../encedo-authenticator-private/JOURNAL.md` — dziennik sesji: co zrobiono, co dalej (poza repo).
- `docs/PLAN-RKV.md` — plan wydania pod nową firmą: decyzje, rebranding, Play, iOS, backend.
- `docs/PARITY.md` — tabela parytetu z v1: co przeniesione, co czeka na test, co pominięte.

Zasady projektu:

- Klucze prywatne i protokół żyją w Ruście; webview dostaje tylko dane do wyświetlenia.
- Backend `api.encedo.com/notify` zostaje bez zmian poza nadawcą push (FCM HTTP v1) i endpointem odświeżania tokena.
- Sekrety (`keystore.jks`, `build.json`, klucze API) nie trafiają do repo.
- Styl UI: tokeny z rkv.pl (sealed = zweryfikowane/Allow, exposed = błąd/Deny), bez gradientów i animacji dekoracyjnych z v1.
- Maszyny: Android buduje `ssh vostro` (`source ~/.android-env.sh`), iOS buduje `ssh macmini` (zsh, PATH z `~/.zshenv`). Szczegóły w `../encedo-authenticator-private/ENVIRONMENT.md`.
- Komunikacja z użytkownikiem po polsku; kod, identyfikatory i commity po angielsku.
