# Lapce volts
This is the source code for the software running on https://plugins.lapce.dev, it allows the users to upload, search and download plugins using the lapce UI.

# Setup a development environment

1. Start database and min.io with docker-compose: `docker-compose up -d`
2. Copy `.env.example` to `.env`, and fill the variables
3. Run the backend with `cargo run --bin volts-server`
4. In another terminal, install trunk if you hadn't installed yet with `cargo install trunk`.
5. Build and serve the frontend: `cd volts-front && trunk serve`
6. Open a browser at https://localhost:3000
7. Be happy coding :tada:!
