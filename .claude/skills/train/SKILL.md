---
name: train
description: Das neuronale Netz trainieren (GA/ES in Rust, ab Phase 6; später Python/RL, Phase 9) und die Gewichte als Asset einbetten. Nutzen, wenn trainiert, ein Netz erneuert oder ein Trainings-Run bewertet werden soll.
---

# NN-Training

## Rust-Track (Phase 6): GA/ES via `snake-train`

```bash
cargo run --release -p snake-train -- --generations 500 --out assets/nn/
```

- Immer `--release` — Training ist simulationsgebunden, Debug ist ~50×
  langsamer.
- Läufe sind lang: als Hintergrundprozess starten, Fortschritt über die
  geloggte Fitness pro Generation verfolgen (sollte monoton-ish steigen;
  stagniert sie >100 Generationen, Mutation/Population prüfen statt einfach
  länger laufen zu lassen).
- Checkpoints mehrerer Generationen behalten (z. B. Gen 10/100/Final) — die
  werden als wählbare Strategien „Neural Net (Gen X)" eingebettet.
- Trainings-Seeds müssen disjunkt von den Benchmark-Seeds sein, sonst ist
  der Benchmark-Vergleich wertlos (Overfitting auf bekannte Futterfolgen).

## Erfolgskontrolle

Nach dem Training den Benchmark laufen lassen (Skill `/benchmark`): Das Netz
muss Greedy deutlich schlagen, sonst Featurevektor/Fitness prüfen.

## Python-Track (Phase 9): DQN/PPO/Behavior Cloning

- Setup unter `python/` (PyO3-Bindings via maturin, stable-baselines3).
- Export ins Rust-Gewichtsformat; danach zwingend den **Roundtrip-Test**
  ausführen (Python-Forward-Pass == Rust-Inferenz auf Testinputs), bevor
  das Netz eingebettet wird — stille Layout-/Transponierungsfehler sind
  hier der häufigste Bug.
