#!/usr/bin/env bash
# Genera icon.icns dai PNG di assets/icons/ con `iconutil` (solo macOS).
#
# Uso: scripts/make-icns.sh [file-di-uscita.icns]     (predefinito: dist/icon.icns)
#
# Mappatura Apple (cartella icon.iconset):
#   icon_16x16.png      <- icon-16.png      icon_16x16@2x.png      <- icon-32.png
#   icon_32x32.png      <- icon-32.png      icon_32x32@2x.png      <- icon-64.png
#   icon_128x128.png    <- icon-128.png     icon_128x128@2x.png    <- icon-256.png
#   icon_256x256.png    <- icon-256.png     icon_256x256@2x.png    <- icon-512.png
#   icon_512x512.png    <- icon-512.png     icon_512x512@2x.png    <- icon-1024.png
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/assets/icons"
OUT="${1:-$ROOT/dist/icon.icns}"

if ! command -v iconutil >/dev/null 2>&1; then
    echo "iconutil non trovato: questo script funziona solo su macOS." >&2
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK:?}"' EXIT
ICONSET="$WORK/icon.iconset"
mkdir -p "$ICONSET"

# <nome nell'iconset>:<PNG sorgente>
MAP=(
    "icon_16x16.png:icon-16.png"
    "icon_16x16@2x.png:icon-32.png"
    "icon_32x32.png:icon-32.png"
    "icon_32x32@2x.png:icon-64.png"
    "icon_128x128.png:icon-128.png"
    "icon_128x128@2x.png:icon-256.png"
    "icon_256x256.png:icon-256.png"
    "icon_256x256@2x.png:icon-512.png"
    "icon_512x512.png:icon-512.png"
    "icon_512x512@2x.png:icon-1024.png"
)

for entry in "${MAP[@]}"; do
    dst="${entry%%:*}"
    src="${entry##*:}"
    if [[ ! -f "$SRC/$src" ]]; then
        echo "PNG mancante: $SRC/$src" >&2
        exit 1
    fi
    cp "$SRC/$src" "$ICONSET/$dst"
done

mkdir -p "$(dirname "$OUT")"
iconutil --convert icns --output "$OUT" "$ICONSET"
echo "Creato: $OUT"
