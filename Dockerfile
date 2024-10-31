FROM rustlang/rust:nightly-bullseye-slim AS builder

WORKDIR /usr/src/mawnolaug

RUN cargo init

COPY Cargo.toml Cargo.lock ./

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked

COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    touch src/main.rs \
    && cargo build --release --locked

FROM ubuntu:latest

COPY --from=builder /usr/src/mawnolaug/target/release/mawnolaug /mawnolaug

CMD ["/mawnolaug"]
