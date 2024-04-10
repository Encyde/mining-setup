FROM rust
COPY ore-cli .
RUN cargo build --release
ENTRYPOINT ["/bin/bash", "-c", "sleep 100"]