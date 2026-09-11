# rust-toolchain.toml sets the Rust version. This tag only supplies rustup.
FROM rust:1-slim-bookworm AS builder
WORKDIR /workspace

# Install the pinned toolchain in its own layer, before the source copy.
COPY rust-toolchain.toml .
RUN rustup toolchain install

COPY . .

RUN cargo build --locked --release

# Runtime stage
FROM debian:bookworm-slim

COPY --from=builder /workspace/target/release/plain_bitassets_app /bin/plain_bitassets_app
COPY --from=builder /workspace/target/release/plain_bitassets_app_cli /bin/plain_bitassets_app_cli

# Verify we placed the binaries in the right place, 
# and that it's executable.
RUN plain_bitassets_app --help
RUN plain_bitassets_app_cli --help

ENTRYPOINT ["plain_bitassets_app"]

