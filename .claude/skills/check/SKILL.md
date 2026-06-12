---
name: check
description: Vollständige Verifikation des Workspace (fmt, clippy, tests, WASM-Check). Vor jedem Commit ausführen, sowie wenn der Nutzer "check", "prüfen" oder "verifizieren" sagt.
---

# Workspace-Check

Führe die Checks in dieser Reihenfolge aus und behebe Fehler sofort, bevor du
weitermachst:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p snake-app --target wasm32-unknown-unknown
```

Hinweise:

- Der WASM-Check ist Pflicht, sobald `snake-app` existiert — Code, der nur
  nativ kompiliert (z. B. Threads, `std::time::Instant` in core-Pfaden),
  fällt sonst erst beim Browser-Build auf.
- `snake-train` und `snake-server` sind rein nativ und brauchen keinen
  WASM-Check; sie sind über `--workspace` bei clippy/test mit abgedeckt.
- Clippy-Findings beheben, nicht per `#[allow]` unterdrücken (Ausnahmen nur
  mit begründendem Kommentar).
- Erst wenn alle vier Schritte sauber durchlaufen, gilt der Stand als
  commit-fähig.
