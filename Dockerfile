FROM rust:1.58-buster as builder

RUN mkdir /hermes /volume
WORKDIR /hermes

# Rust lacks a straightforward way to only install dependencies, so we have to fake the existence
# of the project in order for this to work. The idea is basically to build a separate layer with
# only the dependencies, so that we don't have to reinstall them on every source code change.
# Related issue: https://github.com/rust-lang/cargo/issues/2644
RUN mkdir src && touch src/lib.rs && echo "fn main() {}" > src/cli.rs
COPY ./Cargo.toml .
COPY ./Cargo.lock .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/hermes/target \
    cargo build --release

# Build the source code without installing the dependencies. In order to make rust pick up changes
# in the source files, we have to bump their "date modified".
COPY ./src/ ./src/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/hermes/target \
    find src/ -type f -exec touch {} + \
        && cargo build --release \
        && ls -la ./target/release \
        && cp ./target/release/hermes /volume

FROM gcr.io/distroless/cc
COPY --from=builder /volume/hermes /
ENTRYPOINT ["/hermes"]
