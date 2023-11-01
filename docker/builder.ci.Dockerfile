# syntax=docker/dockerfile:1

FROM rust:bookworm as builder

RUN apt-get update && \
  apt-get install -y build-essential clang cmake protobuf-compiler && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

# On CI we use `docker buildx` with multiple `--platform` arguments, and `--cache-from=type=gha` to cache the layers.
# If we used `--mount=type=cache` here then it looks like the different platforms would be mounted at the same place
# but then be blocked on each other and trying to compile to different targets in parallel.
RUN cargo install --root output --path fendermint/app
