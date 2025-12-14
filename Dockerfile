# SmoothTask Dockerfile - Base image for running SmoothTask daemon
# This Dockerfile creates a minimal container for running the SmoothTask daemon
# with proper cgroup v2 support and system monitoring capabilities.

FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    libsystemd-dev \
    libdbus-1-dev \
    libudev-dev \
    libwayland-dev \
    libxkbcommon-dev \
    libpipewire-0.3-dev \
    libpulse-dev \
    libasound2-dev \
    libevdev-dev \
    libinput-dev \
    libseat-dev \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Clone and build SmoothTask
WORKDIR /app
COPY . .
RUN cargo build --release --bin smoothtaskd

# Create final minimal image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libsystemd0 \
    libdbus-1-3 \
    libudev1 \
    libwayland-client0 \
    libxkbcommon0 \
    libpipewire-0.3-0 \
    libpulse0 \
    libasound2 \
    libevdev2 \
    libinput10 \
    libseat1 \
    && rm -rf /var/lib/apt/lists/*

# Copy built binary
COPY --from=builder /app/target/release/smoothtaskd /usr/local/bin/smoothtaskd

# Create config directory
RUN mkdir -p /etc/smoothtask && \
    mkdir -p /var/log/smoothtask && \
    mkdir -p /var/lib/smoothtask

# Copy example configuration
COPY configs/smoothtask.example.yml /etc/smoothtask/config.yml

# Set up permissions
RUN chmod +x /usr/local/bin/smoothtaskd && \
    chown -R root:root /etc/smoothtask /var/log/smoothtask /var/lib/smoothtask

# Create a non-root user for running the daemon
RUN useradd --system --no-create-home --shell /bin/false smoothtask

# Set up entrypoint
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 8080

USER smoothtask
CMD ["/entrypoint.sh"]