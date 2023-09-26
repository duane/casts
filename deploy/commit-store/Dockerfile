FROM rust:1.72-buster

RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /usr/src/commit-store
COPY . .

RUN cargo build --bin commit-store-server
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1
EXPOSE 5059
ENTRYPOINT ["cargo", "run", "--bin", "commit-store-server"]
