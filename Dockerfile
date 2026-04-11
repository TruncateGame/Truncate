FROM rust:latest AS build

RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    build-essential \
    cmake \
    curl \
    git \
    jq

RUN mkdir -p /tmp/binaryen \
    && curl -L https://github.com/WebAssembly/binaryen/releases/download/version_123/binaryen-version_123-x86_64-linux.tar.gz -o /tmp/binaryen.tar.gz \
    && tar -xzf /tmp/binaryen.tar.gz -C /tmp/binaryen --strip-components=1 \
    && cp -r /tmp/binaryen/bin/* /usr/local/bin/ \
    && cp -r /tmp/binaryen/lib/* /usr/local/lib/ \
    && cp -r /tmp/binaryen/include/* /usr/local/include/ \
    && rm -rf /tmp/binaryen /tmp/binaryen.tar.gz

RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
RUN apt-get install -y nodejs

# Keep this wasm-bindgen-cli version aligned with the version in Cargo.lock
RUN cargo install -f wasm-bindgen-cli --version 0.2.100
RUN rustup target add wasm32-unknown-unknown

RUN mkdir /app
WORKDIR /app

ARG TR_COMMIT
ENV TR_COMMIT=$TR_COMMIT
ARG TR_MSG
ENV TR_MSG=$TR_MSG
ARG TR_ENV
ENV TR_ENV=$TR_ENV
ENV SQLX_OFFLINE="true"

ADD truncate_server /app/truncate_server
ADD truncate_client /app/truncate_client
ADD truncate_dueller /app/truncate_dueller
ADD truncate_core /app/truncate_core
ADD dict_builder /app/dict_builder
ADD Cargo.* /app

RUN cargo build --release -p truncate_server

ADD web_client /app/web_client
ADD .backstage /app/.backstage
RUN chmod +x .backstage/build-web-client.sh
ENV TRUNC_OPT=true
RUN .backstage/build-web-client.sh

# runtime image

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/truncate_server /usr/local/bin/
COPY --from=build /app/web_client/src/_site /app/web_client

RUN mkdir /truncate
ADD word_definitions/defs.db.gz /truncate/defs.db.gz
RUN gunzip /truncate/defs.db.gz

ENV STATIC_DIR=/app/web_client
ENV TR_DEFS_FILE=/truncate/defs.db

CMD truncate_server
