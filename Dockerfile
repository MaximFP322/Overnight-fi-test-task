FROM rust:1.97.0-slim-bookworm AS builder

WORKDIR /build

ARG RUSTFLAGS="-C target-cpu=native"
ARG CARGO_PROFILE_RELEASE_LTO=fat
ARG CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1

ENV RUSTFLAGS="${RUSTFLAGS}" \
    CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO}" \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS="${CARGO_PROFILE_RELEASE_CODEGEN_UNITS}"

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release --bin overnight-fi-test-task \
    && cp target/release/overnight-fi-test-task /build/overnight-fi-test-task

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/overnight-fi-test-task /usr/local/bin/overnight-fi-test-task

STOPSIGNAL SIGINT
ENTRYPOINT ["/usr/local/bin/overnight-fi-test-task"]
