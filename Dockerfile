# Build stage - Download release binaries instead of building from source
FROM alpine:3.22 AS downloader

# Arguments for the release version and target platform
ARG VERSION=v0.1.0-alpha.1
ARG TARGETPLATFORM
ARG TARGETOS=linux
ARG TARGETARCH

# Install download dependencies
RUN apk add --no-cache wget tar

# Download the appropriate binary based on target architecture with retry logic
RUN echo "Downloading text-to-cypher ${VERSION} for ${TARGETPLATFORM} (${TARGETOS}/${TARGETARCH})" && \
    case "${TARGETARCH}" in \
      "amd64") export RUST_ARCH="x86_64-musl" ;; \
      "arm64") export RUST_ARCH="aarch64-musl" ;; \
      *) echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac && \
    DOWNLOAD_URL="https://github.com/barakb/text-to-cypher/releases/download/${VERSION}/text-to-cypher-linux-${RUST_ARCH}.tar.gz" && \
    echo "Download URL: ${DOWNLOAD_URL}" && \
    for i in 1 2 3 4 5; do \
      echo "Download attempt $i/5..." && \
      if wget -O /tmp/text-to-cypher.tar.gz "${DOWNLOAD_URL}"; then \
        echo "✅ Download successful on attempt $i" && \
        break; \
      else \
        echo "❌ Download attempt $i failed" && \
        if [ $i -eq 5 ]; then \
          echo "All download attempts failed" && \
          exit 1; \
        fi && \
        echo "Waiting 10 seconds before retry..." && \
        sleep 10; \
      fi; \
    done && \
    cd /tmp && \
    tar -xzf text-to-cypher.tar.gz && \
    chmod +x text-to-cypher

# Verify the binary works
RUN test -x /tmp/text-to-cypher && echo "Binary verification completed"

# Runtime stage - Use FalkorDB as base image
FROM falkordb/falkordb:latest

# Install runtime dependencies and supervisord
RUN apt-get update && apt-get install -y ca-certificates supervisor && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security (if not already exists)
RUN groupadd -g 1000 appuser 2>/dev/null || true && \
    useradd -m -s /bin/bash -u 1000 -g appuser appuser 2>/dev/null || true

# Set the working directory for our application
WORKDIR /app

# Copy the compiled binary from the downloader stage
COPY --from=downloader /tmp/text-to-cypher /app/text-to-cypher

# Copy the templates from the downloaded package (contains the correct templates for this version)
COPY --from=downloader /tmp/templates ./templates

# Change ownership to the non-root user
RUN chown -R appuser:appuser /app

# Expose the ports your application runs on (in addition to FalkorDB's ports)
EXPOSE 8080 3001

# Copy supervisord configuration and scripts
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY entrypoint.sh /entrypoint.sh

# Create supervisor log directory and make scripts executable
RUN mkdir -p /var/log/supervisor && \
    chmod +x /entrypoint.sh

# Use ENTRYPOINT instead of CMD to ensure it runs
ENTRYPOINT ["/entrypoint.sh"]
