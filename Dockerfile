FROM rust:latest

COPY ./ ./

RUN cargo build --release 

RUN ls -la target/release/

ENTRYPOINT ["./target/release/url_shortener"]
