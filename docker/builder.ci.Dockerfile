# syntax=docker/dockerfile:1

# https://www.docker.com/blog/faster-multi-platform-builds-dockerfile-cross-compilation-guide/
# https://www.docker.com/blog/cross-compiling-rust-code-for-multiple-architectures/
# https://www.docker.com/blog/multi-arch-build-and-images-the-simple-way/
# https://github.com/cross-rs/cross/wiki/Recipes#openssl

# The goal of this step is to copy the `Cargo.toml` and `Cargo.lock` files _without_ the source code,
# so that we can run a step in `builder` that compiles the dependencies only. To do so we first
# copy the whole codebase then get rid of everything except the dependencies and do a build.
# https://stackoverflow.com/questions/49939960/docker-copy-files-using-glob-pattern
# https://stackoverflow.com/questions/58473606/cache-rust-dependencies-with-docker-build/58474618#58474618
FROM --platform=$BUILDPLATFORM ubuntu:latest as stripper

WORKDIR /app

# Copy the Cargo artifacts and Rust sources.
COPY Cargo.toml Cargo.lock ./
COPY fendermint fendermint/

# Delete anything other than cargo files: Rust sources, config files, Markdown, etc.
RUN find fendermint -type f \! -name "Cargo.*" | xargs rm -rf

# Construct dummy sources.
RUN echo "fn main() {}" > fendermint/app/src/main.rs && \
  for crate in $(find fendermint -name "Cargo.toml" | xargs dirname); do \
  touch $crate/src/lib.rs; \
  done

# Using `ubuntu` here because when I try `rust:bookworm` like in `builder.local.Dockerfile` then
# even though I add `aarch64` rustup target as a RUN step, it can't compile `core` later on
# unless that step is repeated in the same command as the cargo build. That doesn't happen
# with the `ubuntu` base and Rust installed.
FROM --platform=$BUILDPLATFORM ubuntu:latest as builder

RUN apt-get update && \
  apt-get install -y build-essential clang cmake protobuf-compiler curl \
  openssl libssl-dev pkg-config

# Get Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
ENV PATH="/root/.cargo/bin:${PATH}"

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
  CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

WORKDIR /app

# Update the version here if our `rust-toolchain.toml` would cause something new to be fetched every time.
ARG RUST_VERSION=1.73
RUN rustup install ${RUST_VERSION} && rustup target add wasm32-unknown-unknown

# Defined here so anything above it can be cached as a common dependency.
ARG TARGETARCH

# Only installing MacOS specific libraries if necessary.
RUN if [ "${TARGETARCH}" = "arm64" ]; then \
  apt-get install -y g++-aarch64-linux-gnu libc6-dev-arm64-cross; \
  rustup target add aarch64-unknown-linux-gnu; \
  rustup toolchain install stable-aarch64-unknown-linux-gnu; \
  fi

# Copy the stripped source code...
COPY --from=stripper /app /app

# ... and build the dependencies.
RUN set -eux; \
  case "${TARGETARCH}" in \
  amd64) ARCH='x86_64'  ;; \
  arm64) ARCH='aarch64' ;; \
  *) echo >&2 "unsupported architecture: ${TARGETARCH}"; exit 1 ;; \
  esac; \
  rustup show ; \
  cargo build --release -p fendermint_app --target ${ARCH}-unknown-linux-gnu

# Now copy the full source...
COPY . .

# ... and do the final build.
# Using `cargo build` instead of `cargo install` because the latter doesn't seem to work with the stripper for some reason.
RUN set -eux; \
  case "${TARGETARCH}" in \
  amd64) ARCH='x86_64'  ;; \
  arm64) ARCH='aarch64' ;; \
  esac; \
  cargo build --release -p fendermint_app --target ${ARCH}-unknown-linux-gnu && \
  mkdir -p output/bin && \
  cp target/${ARCH}-unknown-linux-gnu/release/fendermint output/bin
