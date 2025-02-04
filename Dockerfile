FROM rust:latest

WORKDIR /app

COPY . .

RUN cargo build --release
RUN cargo build --release -p db
RUN cargo build --release -p simple

CMD ["cargo", "run", "--release", "-p", "db"]

# docker build -t recurring-tasks .
# docker run --rm -it -v "$HOME/ca-certificate.crt:/opt/ca-certificate.crt" -e POSTGRES_CONN="$POSTGRES_CONN" -e RUST_LOG=debug recurring-tasks
