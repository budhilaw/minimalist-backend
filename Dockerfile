# Build stage
FROM rust:1.87.0-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Set SQLx to offline mode for builds
ARG SQLX_OFFLINE=true
ENV SQLX_OFFLINE=$SQLX_OFFLINE

# Copy manifests first
COPY Cargo.toml ./
COPY Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN mkdir -p src/bin && echo "fn main() {}" > src/bin/seed.rs

# Build dependencies (this will be cached)
RUN cargo build --release && rm -rf src

# Copy source code, migrations, and SQLx cache
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build the application and seed binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app app

# Create app directory
WORKDIR /app

# Copy the binaries from builder stage
COPY --from=builder /app/target/release/portfolio-backend ./app
COPY --from=builder /app/target/release/seed ./seed

# Copy configuration files (gracefully handle missing files)
COPY example.config.yaml ./.config.yaml
COPY example.secret.yaml ./.secret.yaml

# Copy migrations for runtime access
COPY migrations ./migrations

# Change ownership
RUN chown -R app:app /app

# Switch to app user
USER app

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/api/v1/health || exit 1

# Run the application
CMD ["./app"] 