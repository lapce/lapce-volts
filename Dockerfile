FROM rust:latest as builder

RUN cargo install wasm-pack

WORKDIR /build
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
#    --mount=type=cache,target=/build/target \
    cd ./volts-front && \
    wasm-pack build --target web && \
    cd .. && \
    cargo build --bin volts-server --release

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
COPY ./nginx/volt.png /app/static/volt.png
COPY ./volts-front/assets/tailwind.css /app/static/main.css

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /build/volts-front/pkg/volts_front.js /app/static/main.js
COPY --from=builder /build/volts-front/pkg/volts_front_bg.wasm /app/static/main.wasm
COPY --from=builder /build/target/release/volts-server /app/volts-server

CMD nginx && /app/volts-server