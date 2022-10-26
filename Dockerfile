FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --bin volts-server --release

RUN cargo install wasm-pack
RUN cd /usr/src/app/volts-front && wasm-pack build --target web 

# Runtime image
FROM debian:bullseye-slim

RUN apt-get update
RUN apt-get -y install postgresql-client-13
RUN apt-get -y install nginx

WORKDIR /app
RUN mkdir /app/static

COPY ./nginx/nginx.conf /etc/nginx/nginx.conf
COPY ./nginx/default.conf /etc/nginx/conf.d/default.conf
COPY ./nginx/mime.types /etc/nginx/mime.types
COPY ./nginx/index.html /static/index.html
COPY ./volts-front/assets/tailwind.css /static/main.css

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/volts-front/pkg/volts_front.js /static/main.js
COPY --from=builder /usr/src/app/volts-front/pkg/volts_front_bg.wasm /static/main.wasm
COPY --from=builder /usr/src/app/target/release/volts-server /app/volts-server

RUN /app/volts-server &
CMD nginx