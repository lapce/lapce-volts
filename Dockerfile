FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --bin volts-server --release

# Runtime image
FROM debian:bullseye-slim

RUN apt-get update
RUN apt-get -y install postgresql-client-13
RUN apt-get -y install nginx

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app
RUN mkdir /app/static

COPY ./nginx/default.conf /etc/nginx/conf.d/default.conf
COPY ./nginx/mime.types /etc/nginx/mime.types
COPY ./nginx/index.html /static/index.html
COPY ./volts-front/assets/tailwind.css /static/main.css

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/target/release/volts-server /app/volts-server

RUN /app/volts-server &
CMD nginx