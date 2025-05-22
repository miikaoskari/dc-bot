FROM rust:1.87 AS builder
RUN apt-get update && apt-get install -y \
    cmake
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:unstable-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    yt-dlp
WORKDIR /app
COPY --from=builder /app/target/release/dc-bot /app/
CMD ["./dc-bot"]
