#!/bin/bash
# Script to package Blue Recorder in a Docker container (Ubuntu 22.04).
# This ensures compatibility with older glibc versions across Linux distributions.

set -e

echo "Starting Docker build container for universal AppImage/Debian..."

# We run as root in the container, install dependencies,
# compile using build.sh, and then chown the output directories to the host user
# so you don't get permission errors on the host.
docker run --rm -v "$PWD":/app -w /app ubuntu:22.04 /bin/bash -c "
    export DEBIAN_FRONTEND=noninteractive && \
    apt-get update && \
    apt-get install -y curl build-essential pkg-config libgtk-4-dev libgtk-3-dev libgdk-pixbuf-2.0-dev libglib2.0-dev gettext wget libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev libpipewire-0.3-dev ca-certificates file && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    source \$HOME/.cargo/env && \
    ./build.sh $@ && \
    chown -R $(id -u):$(id -g) build_output/ target/ Cargo.lock
"

echo "Docker build completed successfully."
