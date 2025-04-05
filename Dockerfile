FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/http-proxy-ipv6-pool /main

ENTRYPOINT ["/main"]
