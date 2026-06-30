FROM rust:1.79-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release -p gateway -p mcpgw

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway
COPY --from=builder /app/target/release/mcpgw /usr/local/bin/mcpgw
COPY config/ /etc/mcpgw/config/

ENV MCPGW_CONFIG=/etc/mcpgw/config/default.toml

EXPOSE 8080

CMD ["gateway"]
