---
name: next-phase
description: Den Umsetzungsplan abarbeiten - nächste offene Aufgabe aus plan/01_snake.md finden, umsetzen, abhaken und committen. Nutzen, wenn der Nutzer "weitermachen", "nächste Phase" oder "nächster Schritt" sagt.
---

# Plan abarbeiten

Arbeitszyklus für `plan/01_snake.md`:

1. **Stand ermitteln**: Plan lesen, erste Phase mit offenen Checkboxen
   (`- [ ]`) finden. Phasen werden strikt in Reihenfolge abgearbeitet —
   nicht in spätere Phasen vorgreifen.
2. **Kontext laden**: Den zugehörigen Abschnitt in `docs/concept.md` lesen
   (der Plan verweist teils explizit, z. B. „vgl. Konzept §3.8").
3. **Eine Aufgabe umsetzen**: Pro Arbeitseinheit genau einen Checkbox-Punkt
   (oder eine eng zusammengehörige Gruppe). Spiellogik immer mit Tests in
   `snake-core`.
4. **Verifizieren**: Skill `/check` ausführen; bei UI-Aufgaben zusätzlich
   per `/run-web` das sichtbare Verhalten prüfen.
5. **Abhaken & committen**: Checkbox im Plan auf `- [x]` setzen und die
   Plan-Änderung **mit in denselben Commit** nehmen. Conventional Commit
   (englisch, Imperativ), z. B.
   `feat(core): add hex coordinates and direction enum`.
6. **Phasenabschluss**: Wenn alle Boxen einer Phase abgehakt sind, das
   „Done wenn"-Kriterium explizit gegen die Realität prüfen (nicht nur
   gegen die Checkboxen). Erst dann mit der nächsten Phase beginnen und
   den Phasenwechsel kurz an den Nutzer berichten.

Wenn eine Plan-Aufgabe sich als falsch geschnitten oder überholt
herausstellt: Plan anpassen (eigener `docs:`-Commit mit Begründung), nicht
stillschweigend abweichen.
