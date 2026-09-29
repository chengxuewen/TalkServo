# TalkServo server image — CI-parity + deploy artifact (D11: dev loop is native).
# ubuntu:22.04 = mediasoup's recommended base (widest glibc compat, modules/09).
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl ca-certificates build-essential pkg-config libssl-dev libuv1-dev \
      meson ninja-build python3 python3-pip git \
    && rm -rf /var/lib/apt/lists/*

# rust stable
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . .
RUN cargo build --release -p talkservo-server

EXPOSE 8080/udp
EXPOSE 8080/tcp
# mediasoup RTP/RTCP UDP range (modules/06; narrow in production)
EXPOSE 40000-40100/udp

# TS_JWT_SECRET required (fail-fast); rest optional with defaults
ENV TS_BIND=0.0.0.0:8080
CMD ["./target/release/talkservo-server"]
