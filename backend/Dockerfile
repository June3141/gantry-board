FROM rust:1.93-slim-trixie

WORKDIR /workspace

# Install build dependencies and curl for healthcheck
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reload
RUN cargo install cargo-watch --locked

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/

# Create dummy src to build dependencies
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
RUN cargo build -p gantry-board || true
RUN rm -rf backend/src

# Copy source code
COPY backend/src backend/src
COPY backend/migrations backend/migrations
COPY backend/tests backend/tests

# Build the application
RUN cargo build -p gantry-board

EXPOSE 3000

# Use cargo-watch for development hot reload
CMD ["cargo", "watch", "-w", "backend/src", "-x", "run -p gantry-board"]
