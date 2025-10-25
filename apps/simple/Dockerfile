FROM rust:1.90-trixie AS build
WORKDIR /src
ARG APP_NAME

COPY Cargo.toml Cargo.lock ./
COPY apps/${APP_NAME}/Cargo.toml apps/${APP_NAME}/Cargo.toml

RUN mkdir -p apps/${APP_NAME}/src \
    && echo 'fn main() { println!("stub"); }' > apps/${APP_NAME}/src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p ${APP_NAME} || true

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    CARGO_TARGET_DIR=/cache-target \
    cargo build --release -p ${APP_NAME} && \
    cp /cache-target/release/${APP_NAME} /src/build-artifact

FROM debian:trixie-slim AS runtime
WORKDIR /app
ARG APP_NAME

COPY --from=build /src/build-artifact /app/app

ENV RUST_LOG=debug
ENTRYPOINT ["/app/app"]
