---
name: training-docs
description: Trainings-Anleitung für eine lernende Strategie nach docs/training/<name>/guide.md schreiben oder aktualisieren. Nutzen, sobald eine Strategie mit Trainingslauf implementiert wird (NN/GA, NEAT, Behavior Cloning, DQN, PPO, …) — der Agent macht nur Smoke-Runs, echte Läufe macht der Nutzer auf stärkerer Hardware.
---

# Trainings-Anleitungen schreiben

**Politik**: Echte Trainingsläufe sind rechenintensiv und laufen auf einem
stärkeren Rechner des Nutzers — nicht auf dieser Maschine. Bei der
Implementierung wird nur ein **Smoke-Run** (Miniatur-Parameter, wenige
Sekunden bis Minuten) ausgeführt, um die Pipeline zu verifizieren. Alles,
was der Nutzer für den echten Lauf braucht, steht in
`docs/training/<name>/guide.md` (ein Verzeichnis pro trainierbarer
Strategie, kebab-case, z. B. `mlp-ga/guide.md`, `dqn/guide.md`).

## Pflicht-Struktur der Anleitung (deutsch)

1. **Überblick** — Was wird trainiert, welches Verfahren, was kommt heraus
   (Artefakt-Pfad/Format), wie landet das Ergebnis im Spiel.
2. **Voraussetzungen** — Alle Abhängigkeiten ab frischem System:
   Toolchains, `rustup`-Targets, Python-Setup (uv/venv) falls nötig,
   benötigte Hardware (Kerne/RAM/GPU) und das Checkout/Verzeichnis.
3. **Smoke-Run** — Der Mini-Befehl, der in < ~1 Minute prüft, dass alles
   funktioniert, inkl. erwarteter Ausgabe. Immer zuerst ausführen lassen.
4. **Echter Lauf** — Konkrete Befehlszeile(n) mit empfohlenen Parametern,
   erwartete Laufzeit auf grober Hardware-Klasse, Checkpoint-/Log-Pfade,
   Abbruch-/Fortsetzbarkeit (Resume), Parallelisierung (Threads/rayon).
5. **Hyperparameter** — Tabelle: Name, Default, Wirkung, sinnvoller
   Suchbereich; plus 2–3 konkrete Tuning-Hinweise („wenn Fitness
   stagniert, zuerst X erhöhen").
6. **Auswertung** — Woran erkennt man einen guten Lauf (Benchmark-Befehl,
   Zielwerte, z. B. „⌀-Score > Greedy"), wie vergleicht man Checkpoints.
7. **Ergebnis einchecken** — Wohin die Gewichte kopieren, wie sie
   eingebettet/registriert werden (Dropdown-Eintrag, Assets), welche Tests
   danach laufen müssen.

## Regeln

- Jeder Befehl muss copy-paste-fähig sein und vom Repo-Root ausgehen.
- Reproduzierbarkeit: Seeds dokumentieren; gleiche Parameter + gleicher
  Seed ⇒ gleicher Lauf (sofern das Verfahren das hergibt).
- Die Anleitung gegen die Realität testen: jeden Befehl (mindestens den
  Smoke-Run) einmal selbst ausführen, bevor sie committet wird.
- Bei Änderungen an Trainer-CLI, Formaten oder Pfaden die Anleitung im
  selben Commit aktualisieren.
- In `CLAUDE.md` ist `docs/training/` als Ablageort referenziert — neue
  Dateien dort brauchen keinen weiteren Verweis.
