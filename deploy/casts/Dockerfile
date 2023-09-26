FROM rustlang/rust:nightly-slim

RUN apt-get update && apt-get install -y pkg-config protobuf-compiler libssl-dev
WORKDIR /usr/src/casts
COPY . .

RUN cargo build --bin casts-server
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1
EXPOSE 8243
ENTRYPOINT ["cargo", "run", "--bin", "casts-server"]
