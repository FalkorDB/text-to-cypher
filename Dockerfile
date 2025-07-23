# Build stage
FROM rust:1.88-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev curl

# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src

# Copy the actual source code
COPY src ./src

# Build the actual application
RUN cargo build --release

# Runtime stage
FROM alpine:3.22

# Install runtime dependencies if needed (for your case, you might need ca-certificates for HTTPS)
RUN apk add --no-cache ca-certificates

# Create a non-root user for security
RUN addgroup -g 1000 appuser && \
    adduser -D -s /bin/sh -u 1000 -G appuser appuser

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/text-to-cypher /app/text-to-cypher

# Copy the templates directory
COPY templates ./templates

# Change ownership to the non-root user
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose the ports your application runs on
EXPOSE 8080 3001

# Run the binary
CMD ["./text-to-cypher"]
