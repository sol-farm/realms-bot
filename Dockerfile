FROM rust:1.60.0-slim-buster as BUILDER
RUN apt-get update && apt-get install -y libudev-dev pkg-config build-essential  libssl-dev git
RUN rustup install nightly
RUN cargo install sccache
ENV HOME=/home/root
ENV SCCACHE_CACHE_SIZE="2G"
ENV SCCACHE_DIR=$HOME/.cache/sccache
ENV RUSTC_WRAPPER="/usr/local/cargo/bin/sccache"
WORKDIR $HOME/app
# Copy all files into the docker image
ADD . .
# Start the cache mount and build the cli
RUN --mount=type=cache,target=/home/root/.cache/sccache cargo +nightly build --release --bin cli && cp target/release/cli /tmp/cli
FROM rust:1.60-slim-buster as runtime
COPY --from=BUILDER /tmp/cli /usr/local/bin
ENTRYPOINT ["/usr/local/bin/cli"]