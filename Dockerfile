FROM rust:1.70 as builder

WORKDIR /usr/src/urtask

COPY Cargo.toml Cargo.lock* ./

COPY src ./src

RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/urtask/target/release/urtask /usr/local/bin/urtask

COPY .env* ./

CMD ["urtask"]