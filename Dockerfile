# 1. build the executable
FROM docker.io/rust:1.68-bullseye AS builder
RUN apt update && apt install libssl-dev -y
WORKDIR /godfishbot
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# 2. copy executable and required files to a target container
FROM docker.io/debian:bullseye-slim AS runner
RUN apt update \
    && apt full-upgrade -y \
    && apt install ca-certificates -y \
    && apt autoremove --purge -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /godfishbot/target/release/godfishbot /usr/bin/godfishbot
RUN useradd --create-home --home-dir /godfishbot godfishbot
WORKDIR /godfishbot
COPY res ./res
RUN chown -R godfishbot:godfishbot /godfishbot

USER godfishbot
ENTRYPOINT [ "/usr/bin/godfishbot" ]
