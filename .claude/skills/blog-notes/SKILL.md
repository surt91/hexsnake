---
name: blog-notes
description: Bemerkenswerte Erkenntnisse aus der Entwicklung in docs/blog_notes.md festhalten (Rohmaterial für einen Blog-Post). Nutzen nach jeder Arbeitseinheit mit Aha-Moment, Stolperstein oder Designentscheidung — und immer beim Phasenabschluss.
---

# Blog-Notizen pflegen

`docs/blog_notes.md` sammelt Rohmaterial für einen späteren Blog-Post über
die Entwicklung von HexSnake.

## Was ist notierenswert?

- **Aha-Momente**: nicht offensichtliche Eigenschaften von Hex-Gittern,
  Determinismus, WASM, egui — Dinge, die man vorher nicht wusste.
- **Stolpersteine & API-Fallen**: veraltete Tutorials, überraschende
  Breaking Changes, Bugs mit interessanter Ursache (inkl. wie sie gefunden
  wurden).
- **Designentscheidungen mit Begründung**: warum dieser Weg und nicht der
  naheliegende andere (z. B. „kein HashSet wegen Iterationsreihenfolge").
- **Schöne Tests**: Property-Tests oder Testtricks, die Fehlerklassen
  abdecken, die Beispieltests verfehlen.
- **Zahlen & Ergebnisse**: Benchmark-Resultate, Binary-Größen,
  Trainingskurven — alles, was später Diagramme oder konkrete Belege liefert.

**Nicht** notieren: Routinearbeit, reine Plan-Wiedergabe, Dinge die schon in
`docs/concept.md` stehen.

## Format

- Ein Bullet pro Erkenntnis, unter der passenden Phasen-Überschrift
  (`## Phase N — Titel`), neue Überschrift beim ersten Eintrag einer Phase.
- Deutsch, mit genug Kontext, dass es nach Monaten noch verständlich ist;
  Fettdruck als Mini-Titel des Bullets.
- Notizen gehören in denselben Commit wie die Arbeit, aus der sie stammen
  (oder einen unmittelbar folgenden `docs:`-Commit).

## Wann?

Direkt nach jeder Arbeitseinheit kurz prüfen: „War hier etwas dabei, das in
einen Blog-Post gehört?" — wenn ja, sofort notieren, solange der Kontext
frisch ist. Spätestens beim Phasenabschluss die Phase noch einmal
durchdenken.
