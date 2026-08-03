## All in one, multi stage compile + runtime Docker build for your architecture.
## cargo-chef: deps layer cached when Cargo.lock unchanged.

FROM docker.io/lukemathwalker/cargo-chef:latest-rust-1.97.1-trixie AS chef
RUN cargo install cargo-strip --locked
WORKDIR /builder

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY ./lib ./lib
COPY ./client/core/rs ./client/core/rs
COPY ./client/periphery ./client/periphery
COPY ./bin/periphery ./bin/periphery
COPY ./xtask ./xtask
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /builder/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package komodo_periphery
COPY Cargo.toml Cargo.lock ./
COPY ./lib ./lib
COPY ./client/core/rs ./client/core/rs
COPY ./client/periphery ./client/periphery
COPY ./bin/periphery ./bin/periphery
COPY ./xtask ./xtask
RUN cargo build -p komodo_periphery --release && cargo strip

# Final Image
FROM debian:trixie-slim

COPY ./bin/periphery/starship.toml /starship.toml
COPY ./bin/periphery/debian-deps.sh .
RUN sh ./debian-deps.sh && rm ./debian-deps.sh

COPY --from=builder /builder/target/release/periphery /usr/local/bin/periphery

COPY ./bin/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 8120

# Can mount config file to /config/*config*.toml and it will be picked up.
ENV PERIPHERY_CONFIG_PATHS="/config"
# Change the default in container to /config/keys to match Core
ENV PERIPHERY_PRIVATE_KEY="file:/config/keys/periphery.key"

ENTRYPOINT [ "entrypoint.sh" ]
CMD [ "periphery" ]

# Label to prevent Komodo from stopping with StopAllContainers
LABEL komodo.skip="true"
# Label for ghcr
LABEL org.opencontainers.image.source="https://github.com/moghtech/komodo"
LABEL org.opencontainers.image.description="Komodo Periphery"
LABEL org.opencontainers.image.licenses="GPL-3.0"
