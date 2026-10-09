#!/usr/bin/env bash
# Dipendenze di sviluppo per compilare Hasher su Debian / Ubuntu.
# (libgtk-3-dev serve a rfd per i dialoghi; Mesa + Xvfb per gli screenshot headless.)
set -euo pipefail

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
    SUDO="sudo"
fi

$SUDO apt-get update
$SUDO apt-get install -y \
    build-essential pkg-config \
    libgtk-3-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libgl1-mesa-dri \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libssl-dev \
    xvfb

echo "Dipendenze installate. Ora: cargo build --release"
