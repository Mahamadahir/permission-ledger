FROM rust:1.82-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
COPY crates ./crates
COPY worker ./worker
RUN cargo build --release -p permission-ledger-backend

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/permission-ledger-backend /usr/local/bin/permission-ledger-backend
CMD ["permission-ledger-backend"]
