FROM rust:latest AS builder

WORKDIR /usr/src/url_shortener
COPY ./ ./
RUN cargo run --release -- config

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/url_shortener/target/release/url_shortener /usr/local/bin/url_shortener
COPY --from=builder /usr/src/url_shortener/config.toml /usr/local/config.toml
COPY --from=builder /usr/src/url_shortener/html /usr/local/html

WORKDIR /usr/local/

ENTRYPOINT ["/usr/local/bin/url_shortener"]
