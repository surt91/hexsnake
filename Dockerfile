# All-in-one image: the axum server serves both the global-highscore API and
# the static WASM frontend, so the complete game ships as a single container.
#
#   docker build -t hexsnake .
#   docker run --rm -p 8080:8080 -v hexsnake-data:/data hexsnake
#
# (see docker-compose.yml for the hardened, recommended setup).

# ---- builder: WASM frontend + native server in one cached layer ------------
FROM rust:1-bookworm AS builder
WORKDIR /build

RUN rustup target add wasm32-unknown-unknown \
 && cargo install --locked trunk

COPY . .

# Frontend is served same-origin from the server root, so public-url is "/".
# SNAKE_SERVER_URL stays unset ⇒ the client talks to the same origin.
RUN cd crates/snake-app && trunk build --release --public-url /

RUN cargo build --release -p snake-server

# ---- runtime: slim, non-root, read-only-friendly ---------------------------
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --gid nogroup snake \
 && mkdir -p /data \
 && chown 10001:nogroup /data

COPY --from=builder /build/target/release/snake-server /usr/local/bin/snake-server
COPY --from=builder /build/crates/snake-app/dist /app/static

ENV STATIC_DIR=/app/static \
    DB_PATH=/data/highscores.db \
    BIND_ADDR=0.0.0.0:8080

EXPOSE 8080
# A fresh named volume mounted at /data inherits this ownership, so the
# non-root user can write the SQLite database even with a read-only rootfs.
VOLUME ["/data"]
USER 10001
ENTRYPOINT ["snake-server"]
