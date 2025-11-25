# Stage 1: Base image with cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-1.88.0 AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
	lld clang \
	protobuf-compiler \
	libprotobuf-dev \
	&& cargo install sccache \
	&& rm -rf /var/lib/apt/lists/* /var/cache/apt/*
WORKDIR /app

# Stage 2: Planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

# Build profile, release by default
ENV RUSTC_WRAPPER=sccache

# Build and cache dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the full project for final build
COPY . .

# Build the application
RUN cargo build --release --locked --bin sonar

# Copy the binary to a location that can be accessed by the runtime stage
RUN cp /app/target/release/sonar /app/sonar

# Stage 4: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
	ca-certificates \
	openssl \
	libssl3 \
	&& rm -rf /var/lib/apt/lists/* /var/cache/apt/*

# Copy the built binary from the builder stage
COPY --from=builder /app/sonar /usr/local/bin/sonar

# Expose the application port
EXPOSE 8080

# Set entrypoint
ENTRYPOINT ["sonar"]