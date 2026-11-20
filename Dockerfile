# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM rust:1-bookworm AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 sqlite3 openssl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/party-hub /usr/local/bin/party-hub
COPY pages ./pages
COPY static ./static
COPY createAuthor.sh ./createAuthor.sh

RUN chmod 0755 /usr/local/bin/party-hub ./createAuthor.sh \
    && mkdir -p /data

ENV PUBLIC_VAPID_KEY_PATH=/run/party-hub/public_vapid_key.pem \
    PRIVATE_VAPID_KEY_PATH=/run/party-hub/private_vapid_key.pem \
    SQLITE_DB_PATH=/data/party.db \
    WEB_ASSETS_PATH=/app \
    PORT=8080 \
    ENV=prod

EXPOSE 8080

CMD ["/usr/local/bin/party-hub"]
