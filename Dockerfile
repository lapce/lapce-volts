FROM rust:latest as builder

RUN USER=root cargo new app
WORKDIR /usr/src/app
RUN cargo install wasm-pack

RUN mkdir volts-back
RUN mkdir volts-core
RUN mkdir volts-cli
RUN mkdir volts-front
COPY Cargo.toml Cargo.lock ./
COPY ./volts-back/Cargo.toml ./volts-back/
COPY ./volts-core/Cargo.toml ./volts-core/
COPY ./volts-front/Cargo.toml ./volts-front/
# Needs at least a main.rs file with a main function
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN mkdir -p volts-back/src/bin && echo "fn main(){}" > volts-back/src/bin/server.rs && touch volts-back/src/lib.rs
RUN mkdir volts-core/src && touch volts-core/src/lib.rs
RUN mkdir -p volts-front/src/bin && echo "fn main(){}" > volts-front/src/bin/front.rs && touch volts-front/src/lib.rs
# Will build all dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --bin volts-server --release && cd /usr/src/app/volts-front && wasm-pack build --target web
RUN cd /usr/src/app
RUN rm src/main.rs

COPY ./volts-back ./volts-back
COPY ./volts-core ./volts-core
COPY ./volts-front ./volts-front
RUN cargo build --bin volts-server --release

RUN cd /usr/src/app/volts-front && wasm-pack build --target web 

# Runtime image
FROM debian:bullseye

RUN apt-get update
RUN apt-get -y install postgresql-client-13
RUN apt-get -y install nginx
RUN apt-get install ca-certificates -y
RUN update-ca-certificates

WORKDIR /app
RUN mkdir /app/static

COPY ./nginx/nginx.conf /etc/nginx/nginx.conf
COPY ./nginx/default.conf /etc/nginx/conf.d/default.conf
COPY ./nginx/mime.types /etc/nginx/mime.types
COPY ./nginx/index.html /app/static/index.html
COPY ./volts-front/assets/tailwind.css /app/static/main.css

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/volts-front/pkg/volts_front.js /app/static/main.js
COPY --from=builder /usr/src/app/volts-front/pkg/volts_front_bg.wasm /app/static/main.wasm
COPY --from=builder /usr/src/app/target/release/volts-server /app/volts-server

CMD nginx && /app/volts-server