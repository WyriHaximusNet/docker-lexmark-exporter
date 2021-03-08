FROM rust:1 AS factory
WORKDIR /opt/app
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=factory /usr/local/cargo/bin/lexmark-exporter /usr/local/bin/lexmark-exporter
CMD ["lexmark-exporter"]
