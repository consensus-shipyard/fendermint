# syntax=docker/dockerfile:1

# https://www.docker.com/blog/faster-multi-platform-builds-dockerfile-cross-compilation-guide/
# https://www.docker.com/blog/cross-compiling-rust-code-for-multiple-architectures/
# https://www.docker.com/blog/multi-arch-build-and-images-the-simple-way/
# https://github.com/cross-rs/cross/wiki/Recipes#openssl

FROM --platform=$BUILDPLATFORM ubuntu:latest as builder

ARG TARGETARCH

RUN apt-get update && \
  apt-get install -y build-essential clang cmake protobuf-compiler curl \
  g++-aarch64-linux-gnu libc6-dev-arm64-cross \
  openssl libssl-dev pkg-config \
  && \
  rm -rf /var/lib/apt/lists/*

# Get Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add aarch64-unknown-linux-gnu
RUN rustup toolchain install stable-aarch64-unknown-linux-gnu

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
  CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

# ENV SSL_VERSION=1.1.1

# RUN curl https://www.openssl.org/source/openssl-$SSL_VERSION.tar.gz -O && \
#   tar -xzf openssl-$SSL_VERSION.tar.gz && \
#   cd openssl-$SSL_VERSION && ./config && make depend && make install && \
#   cd .. && rm -rf openssl-$SSL_VERSION*

# # https://github.com/sfackler/rust-openssl/issues/766
# # https://askubuntu.com/questions/729213/missing-opensslconf-h-when-compiling-git
# ENV OPENSSL_STATIC=1 \
#   OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu \
#   OPENSSL_INCLUDE_DIR=/usr/include/openssl \
#   AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR=/usr/lib/aarch64-linux-gnu \
#   AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR=/usr/include/openssl

# # There are include files at /urs/local/include/openssl which contain opensslconf.h,
# # but something still looks for it in /urs/include/openssl, and configuration.h only exists there.
# RUN cd /usr/include/openssl && \
#   ln -s /usr/include/x86_64-linux-gnu/openssl/opensslconf.h opensslconf.h && \
#   ln -s /usr/include/x86_64-linux-gnu/openssl/configuration.h configuration.h

WORKDIR /app

COPY . .

# On CI we use `docker buildx` with multiple `--platform` arguments, and `--cache-from=type=gha` to cache the layers.
# If we used `--mount=type=cache` here then it looks like the different platforms would be mounted at the same place
# and then one of them can get blocked trying to acquire a lock on the build directory.

RUN set -eux; \
  case "${TARGETARCH}" in \
  amd64) ARCH='x86_64'  ;; \
  arm64) ARCH='aarch64' ;; \
  *) echo >&2 "unsupported architecture: ${TARGETARCH}"; exit 1 ;; \
  esac; \
  rustup show ; \
  cargo install --root output --path fendermint/app --target ${ARCH}-unknown-linux-gnu
