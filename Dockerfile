FROM rust:1.56.0-buster as builder
WORKDIR /usr/src/munje
COPY . .
RUN cargo build --release

FROM debian:buster-slim
LABEL Name=munje Version=0.0.1
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/munje/target/release/munje /app/munje
CMD ["/app/munje"]
