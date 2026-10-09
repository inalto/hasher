#!/usr/bin/env bash
# Dipendenze di sviluppo per compilare Hasher su AlmaLinux / RHEL / Rocky 9.
# (gtk3-devel serve a rfd per i dialoghi; mesa + Xvfb per gli screenshot headless.)
set -euo pipefail

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
    SUDO="sudo"
fi

$SUDO dnf install -y \
    gcc pkgconf-pkg-config \
    gtk3-devel libxkbcommon-devel wayland-devel mesa-libGL-devel mesa-dri-drivers \
    xorg-x11-server-Xvfb

echo "Dipendenze installate. Ora: cargo build --release"
