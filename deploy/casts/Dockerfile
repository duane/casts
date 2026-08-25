# Build context is the PARENT directory holding both sibling checkouts
# (casts/ and commit-store/), because casts depends on commit-store by the
# relative path ../../commit-store. compose.yaml sets this up, and
# Dockerfile.dockerignore keeps the context from including either target/.
FROM rust:1-slim-bookworm AS builder

# libprotobuf-dev, not just protobuf-compiler: on bookworm the well-known
# types (google/protobuf/timestamp.proto) ship in the -dev package, and
# without it the build fails with "timestamp.proto: File not found".
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config protobuf-compiler libprotobuf-dev libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src
COPY casts/ casts/
COPY commit-store/ commit-store/

WORKDIR /usr/src/casts
# Only the gRPC server ships. The graphql feature pulls juniper and a second
# hyper major version for a binary nothing deployed serves.
RUN cargo build --release --bin casts-grpc-server

FROM debian:bookworm-slim
# libssl3 named explicitly rather than relied on as a transitive of
# ca-certificates.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/casts/target/release/casts-grpc-server /usr/local/bin/
COPY casts/deploy/casts/docker-entrypoint.sh /usr/local/bin/

RUN useradd --system --create-home --shell /usr/sbin/nologin casts
USER casts

ENV RUST_LOG=info
EXPOSE 8243

# The entrypoint converts env-delivered PEM content into files (see the script
# for why); CMD stays separate so an ECS containerOverride can swap the binary
# without also having to override the entrypoint.
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["casts-grpc-server"]
