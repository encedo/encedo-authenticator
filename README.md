# Encedo Mobile Authenticator

The phone half of every sensitive operation on an Encedo HEM: the module asks,
the phone signs the answer. Android and iOS, built with Tauri 2, Rust and Svelte 5.

- `main`: the v1 Cordova app as published on Google Play (1.0.3). Reference only.
- `v2`: this rewrite. Start with [`docs/`](docs/): the plan, the protocol, the
  machines, and the session journal.

## Run it

```sh
npm install
npm run dev                 # frontend alone in a browser, http://localhost:1420
npm run dev -- --host       # same, reachable from a phone on the LAN
cargo tauri dev             # desktop window with the Rust core
cargo tauri android dev     # on vostro, with the emulator running
```

While the mockup lasts, `?demo=<screen>&theme=dark` opens any screen directly
(`src/lib/demo.ts` lists them) and the Settings screen can raise a pretend
request. Style tokens and voice come from
`encedo-manager/design/STYLE.md`.

Secrets (`keystore.jks`, `build.json`, Firebase configs) stay outside the repo.
