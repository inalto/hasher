#!/usr/bin/env bash
# Crea dist/hasher-<versione>-linux-<arch>.tar.gz a partire dal binario release.
#
# Uso:   scripts/package-linux.sh <versione> [percorso-binario]
# Esempio: cargo build --release && scripts/package-linux.sh 0.1.0
#
# Contenuto dell'archivio (dentro la cartella hasher-<versione>-linux-<arch>/):
#   hasher            eseguibile
#   README.md
#   hasher.desktop    voce di menu (opzionale)
#   icon.png          icona 256x256 (opzionale)
#   LICENSES-fonts/   licenze OFL dei font incorporati
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "Uso: $0 <versione> [percorso-binario]" >&2
    exit 2
fi

VERSION="${1#v}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${2:-$ROOT/target/release/hasher}"
ARCH="$(uname -m)"
NAME="hasher-${VERSION}-linux-${ARCH}"
DIST="$ROOT/dist"

for f in "$BIN" \
         "$ROOT/README.md" \
         "$ROOT/assets/linux/hasher.desktop" \
         "$ROOT/assets/icons/icon-256.png"; do
    if [[ ! -f "$f" ]]; then
        echo "File mancante: $f" >&2
        exit 1
    fi
done

STAGE="$(mktemp -d)"
trap 'rm -rf "${STAGE:?}"' EXIT

PKG="$STAGE/$NAME"
mkdir -p "$PKG/LICENSES-fonts"
install -m 755 "$BIN" "$PKG/hasher"
install -m 644 "$ROOT/README.md" "$PKG/README.md"
install -m 644 "$ROOT/assets/linux/hasher.desktop" "$PKG/hasher.desktop"
install -m 644 "$ROOT/assets/icons/icon-256.png" "$PKG/icon.png"
install -m 644 "$ROOT"/assets/fonts/LICENSE-*.txt "$PKG/LICENSES-fonts/"

mkdir -p "$DIST"
rm -f "${DIST:?}/${NAME:?}.tar.gz"
tar --owner=0 --group=0 --numeric-owner -C "$STAGE" -czf "$DIST/$NAME.tar.gz" "$NAME"

echo "Creato: $DIST/$NAME.tar.gz"
