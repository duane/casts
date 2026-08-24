# Build context is the PARENT directory holding both sibling checkouts
# (casts/ and commit-store/), because casts depends on commit-store by the
# relative path ../../commit-store. compose.yaml sets this up.
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config protobuf-compiler libprotobuf-dev libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src
COPY casts/ casts/
COPY commit-store/ commit-store/

WORKDIR /usr/src/casts
RUN cargo build --release --bin casts-grpc-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/casts/target/release/casts-grpc-server /usr/local/bin/
ENV RUST_LOG=info
EXPOSE 8243
ENTRYPOINT ["casts-grpc-server"]
