# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.76.0
ARG ALPINE_VERSION=3.19
ARG APP_NAME=postgres-db-actions

################################################################################
# Create a stage for building the application.
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk update && apk upgrade --no-cache && apk add --no-cache build-base clang lld musl-dev git file pkgconfig openssl-dev libcrypto3 libssl3 openssl-libs-static postgresql16-client && rm -rf /var/cache/apk/*

# Copy Cargo.toml and Cargo.lock to cache dependencies.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && touch src/main.rs && cargo fetch --locked

# Copy the source code.
COPY src ./src

# Build the application.
RUN cargo build --locked --release --target-dir ./target && cp ./target/release/${APP_NAME} /bin/${APP_NAME}


FROM alpine:${ALPINE_VERSION} AS final
ARG APP_NAME
ENV APP_NAME=${APP_NAME}

LABEL maintainer="Cristian Iordachescu <cristian.iordachescu@ivfuture.uk>"
LABEL version="0.2.0"
LABEL title="Postgres actions cli"
LABEL description="This is a Dockerfile for running postgres-db-actions. For more information visit run with --help."

RUN apk update && apk upgrade --no-cache && apk add --no-cache bash postgresql16-client ca-certificates && rm -rf /var/cache/apk/*
SHELL [ "/bin/bash", "-c" ]

# COPY ./certs/CAs/rootCA.crt /usr/local/share/ca-certificates/
# RUN update-ca-certificates

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
ARG UID=1000
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/pghome" \
    --shell "/sbin/nologin" \
    --uid "${UID}" \
    appuser
USER appuser

WORKDIR /pghome

# Copy the executable from the "build" stage.
COPY --from=build /bin/${APP_NAME} /bin/

ENV PG_HOSTNAME="localhost"
ENV PG_SUPERUSER="postgres"
ENV PG_PASS="postgres"
ENV PG_PORT="5432"
ENV S3_ENDPOINT="http://localhost:9000"
ENV S3_ACCESS_KEY="minioadmin"
ENV S3_SECRET_KEY="minioadmin"
ENV S3_BUCKET="postgres-backups"
ENV S3_REGION="eu-east-1"
ENV S3_PREFIX="backups"

# What the container should run when it is started.
ENTRYPOINT ["/bin/postgres-db-actions"]
