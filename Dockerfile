FROM rust:latest AS builder

WORKDIR /usr/src/url_shortener
COPY ./ ./
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/url_shortener/target/release/url_shortener /usr/local/bin/url_shortener

ENTRYPOINT ["/usr/local/bin/url_shortener"]
