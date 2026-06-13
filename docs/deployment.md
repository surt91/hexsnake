# Deployment

HexSnake läuft **immer voll offline** im Browser (GitHub Pages) und nativ.
Der optionale `snake-server` (axum + SQLite) liefert globale Highscores und
die tägliche Challenge. Dieses Dokument beschreibt drei Hosting-Varianten.

Das primäre Hosting bleibt **GitHub Pages**
(<https://surt91.github.io/hexsnake/>); ein Server ist nur nötig, wenn man
globale Bestenlisten möchte.

## Überblick: das All-in-one-Image

`Dockerfile` baut in einem Multi-Stage-Build

1. das WASM-Frontend (`trunk build --release --public-url /`) und
2. den Server (`cargo build --release -p snake-server`),

und packt beides in ein schlankes Runtime-Image. Der Server liefert über
`tower-http`/`ServeDir` neben der API auch die statischen Dateien aus — das
ganze Spiel inkl. Highscore-Server ist damit **ein** Container.

```bash
docker compose up -d --build
# -> http://localhost:8080
```

Die SQLite-Datei liegt auf dem Volume `/data` (Pfad per `DB_PATH`). Das
Image läuft als Non-Root-User (UID 10001), `docker-compose.yml` setzt
zusätzlich Root-Filesystem read-only (`read_only: true`, beschreibbar nur
`/data` und ein `tmpfs` auf `/tmp`), `cap_drop: ALL` und
`no-new-privileges`.

### Konfiguration (Umgebungsvariablen)

| Variable | Default | Zweck |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | Listen-Adresse |
| `DB_PATH` | `/data/highscores.db` | SQLite-Datei |
| `STATIC_DIR` | `/app/static` | ausgeliefertes Frontend (leer ⇒ API-only) |
| `MAX_BODY_BYTES` | `524288` | Body-Size-Limit |
| `MAX_INPUTS` | `100000` | max. Inputlistenlänge je Lauf |
| `MAX_TICKS` | `2000000` | Tick-Obergrenze der Re-Simulation |
| `MAX_NAME_LEN` | `20` | max. Namenslänge |
| `RATE_LIMIT_PER_MIN` | `20` | POSTs pro IP/Minute (`0` = aus) |
| `VERIFY_CONCURRENCY` | `4` | gleichzeitige Re-Simulationen (Semaphore) |
| `TRUST_FORWARDED_FOR` | `0` | Client-IP aus `X-Forwarded-For` lesen |
| `CORS_ALLOW_ORIGIN` | – | erlaubte CORS-Origins (Komma-getrennt) |
| `DAILY_SECRET` | fix | Geheimnis für den Tagesseed |

> **`TRUST_FORWARDED_FOR` nur hinter einem Proxy/Tunnel aktivieren**, den du
> selbst kontrollierst. Sonst kann ein Client das Rate-Limit durch einen
> gefälschten Header umgehen.

---

## Variante 1 — Docker auf einem VPS

1. Repo klonen, `docker compose up -d --build`.
2. **Reverse Proxy für HTTPS** davorsetzen. Beispiel mit Caddy
   (`Caddyfile`):

   ```caddy
   hexsnake.example.com {
       reverse_proxy localhost:8080
   }
   ```

   Caddy holt das Zertifikat automatisch (Let's Encrypt). Da der Proxy
   `X-Forwarded-For` setzt, im Compose `TRUST_FORWARDED_FOR=1` aktivieren,
   damit das Rate-Limiting die echte Client-IP sieht.
3. Backup: regelmäßig das Volume `hexsnake-data` (bzw. `/data`) sichern.

---

## Variante 2 — Heimserver via Cloudflare Tunnel (empfohlen)

`cloudflared` als **Sidecar-Container** — kein offener Port, die Heim-IP
bleibt verborgen, funktioniert auch hinter DS-Lite/CGNAT.

1. In Cloudflare Zero Trust einen Tunnel anlegen, das Token kopieren und als
   `TUNNEL_TOKEN` hinterlegen. Den Public Hostname des Tunnels auf
   `http://hexsnake:8080` zeigen lassen.
2. Sidecar in eine `docker-compose.override.yml` (oder direkt ins Compose):

   ```yaml
   services:
     cloudflared:
       image: cloudflare/cloudflared:latest
       command: tunnel --no-autoupdate run
       environment:
         TUNNEL_TOKEN: ${TUNNEL_TOKEN}
       restart: unless-stopped
       depends_on:
         - hexsnake
   ```

3. Cloudflare terminiert HTTPS und setzt `X-Forwarded-For`, daher am Server
   `TRUST_FORWARDED_FOR=1` setzen.

**Falls doch Portforwarding** statt Tunnel: nur 443 (+80) weiterleiten, den
Host in ein eigenes VLAN/DMZ isolieren, **SSH nie exponieren** (Administration
nur über LAN/VPN), automatische Updates (unattended-upgrades, regelmäßige
Image-Rebuilds) und Backups des SQLite-Volumes einrichten.

---

## Variante 3 — GitHub Pages + externer API-Server

Das Frontend bleibt auf GitHub Pages, die API läuft separat (z. B. Variante 1
oder 2).

1. **Server-URL zur Buildzeit setzen.** Der Client liest `SNAKE_SERVER_URL`
   beim Kompilieren (`option_env!`). Im Pages-Workflow als Repo-Variable
   `SNAKE_SERVER_URL` (Settings → Secrets and variables → Actions →
   *Variables*) hinterlegen; der Build reicht sie an `trunk` weiter
   (siehe `.github/workflows/deploy.yml`). Ohne Wert spricht der Pages-Build
   keinen Server an und bleibt rein lokal.
2. **CORS:** am Server `CORS_ALLOW_ORIGIN=https://<user>.github.io` setzen,
   damit der Browser die Cross-Origin-Requests erlaubt.
3. **HTTPS ist Pflicht:** GitHub Pages läuft über HTTPS, und ein Browser
   **blockiert Mixed Content** zu einem HTTP-Server. Der API-Server muss
   also ebenfalls über HTTPS erreichbar sein (Reverse Proxy oder Tunnel,
   siehe oben).

---

## Verhalten ohne Server / offline

- Ist keine `SNAKE_SERVER_URL` gesetzt und läuft das Spiel nicht same-origin
  hinter dem Server, bleibt das globale UI ausgeblendet — alles funktioniert
  rein lokal.
- Fällt der Server zur Laufzeit aus, schaltet der Client still auf die
  lokalen Tabellen zurück. Offline gespielte Läufe werden lokal gemerkt und
  später automatisch nachgereicht (gedrosselt).
- **Mit gezogenem Netzwerkstecker verhält sich das Spiel exakt wie der
  reine Offline-Build.**
