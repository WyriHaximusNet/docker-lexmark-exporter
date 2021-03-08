FROM rust:1 AS factory
WORKDIR /opt/app
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
# hadolint ignore=DL3008
RUN apt-get update && apt-get install -y libssl-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*
COPY --from=factory /usr/local/cargo/bin/lexmark-exporter /usr/local/bin/lexmark-exporter
CMD ["lexmark-exporter"]
