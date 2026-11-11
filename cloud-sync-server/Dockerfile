FROM rust:1.86-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release --locked

FROM builder AS test
RUN cargo test --locked

FROM alpine:3.21 AS runtime
RUN addgroup -S cloudsync \
 && adduser -S -G cloudsync cloudsync \
 && mkdir -p /data \
 && chown cloudsync:cloudsync /data
COPY --from=builder /src/target/release/cloud-sync-server /usr/local/bin/cloud-sync-server
USER cloudsync
VOLUME ["/data"]
ENV SYNC_BIND=0.0.0.0:8080 SYNC_DATA_DIR=/data SYNC_MAX_BODY_BYTES=67108864
EXPOSE 8080 8081
ENTRYPOINT ["/usr/local/bin/cloud-sync-server"]
