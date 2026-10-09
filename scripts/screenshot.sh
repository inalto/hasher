#!/usr/bin/env bash
# Screenshot headless di Hasher (Linux): avvia Xvfb se serve e lancia il binario di debug con il
# rendering OpenGL software di Mesa.
#
# Uso:   scripts/screenshot.sh <out.png> [argomenti per hasher...]
# Esempi:
#   scripts/screenshot.sh /tmp/vuoto.png
#   HASHER_THEME=dark scripts/screenshot.sh /tmp/lista.png file1.iso file2.iso
#   scripts/screenshot.sh /tmp/lento.png --screenshot-frames 120 grosso.bin
#
# Variabili d'ambiente:
#   HASHER_THEME=light|dark|system   forza il tema (altrimenti quello delle impostazioni)
#   HASHER_BIN=<percorso>     binario da usare (predefinito: target/debug/hasher)
#   XVFB_DISPLAY=<n>          numero del display virtuale (predefinito: 99)
#   XVFB_SCREEN=<WxHxD>       geometria dello schermo virtuale (predefinito: 1280x800x24)
#
# Le impostazioni dell'app puntano a un file temporaneo (HASHER_SETTINGS_PATH): quelle reali
# non vengono lette né modificate. Il comando è limitato a 90 secondi.
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Uso: $0 <out.png> [argomenti per hasher...]" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${HASHER_BIN:-$ROOT/target/debug/hasher}"
DISPLAY_NUM="${XVFB_DISPLAY:-99}"
SCREEN="${XVFB_SCREEN:-1280x800x24}"

OUT="$1"
shift
# Percorso assoluto, così non dipende dalla cartella corrente.
mkdir -p "$(dirname "$OUT")"
OUT="$(cd "$(dirname "$OUT")" && pwd)/$(basename "$OUT")"

if [[ ! -x "$BIN" ]]; then
    echo "Binario non trovato: $BIN (esegui prima: cargo build)" >&2
    exit 1
fi

case "${HASHER_THEME:-}" in
    "" | light | dark | system) ;;
    *)
        echo "HASHER_THEME deve essere 'light', 'dark' o 'system' (ricevuto: ${HASHER_THEME})" >&2
        exit 2
        ;;
esac

XVFB_PID=""
TMP="$(mktemp -d)"
cleanup() {
    if [[ -n "$XVFB_PID" ]]; then
        kill "$XVFB_PID" 2>/dev/null || true
    fi
    rm -rf "${TMP:?}"
}
trap cleanup EXIT

# Avvia Xvfb solo se il display non è già servito.
if ! pgrep -f "Xvfb :${DISPLAY_NUM}( |\$)" >/dev/null 2>&1; then
    if ! command -v Xvfb >/dev/null 2>&1; then
        echo "Xvfb non trovato (vedi scripts/dev-deps-el9.sh o scripts/dev-deps-debian.sh)" >&2
        exit 1
    fi
    Xvfb ":${DISPLAY_NUM}" -screen 0 "$SCREEN" -nolisten tcp >/dev/null 2>&1 &
    XVFB_PID=$!
    for _ in $(seq 1 50); do
        [[ -S "/tmp/.X11-unix/X${DISPLAY_NUM}" ]] && break
        sleep 0.1
    done
    if [[ ! -S "/tmp/.X11-unix/X${DISPLAY_NUM}" ]]; then
        echo "Xvfb :${DISPLAY_NUM} non si è avviato" >&2
        exit 1
    fi
fi

rm -f "${OUT:?}"
set +e
env -u WAYLAND_DISPLAY \
    DISPLAY=":${DISPLAY_NUM}" \
    LIBGL_ALWAYS_SOFTWARE=1 \
    HASHER_SETTINGS_PATH="$TMP/hasher.settings.json" \
    ${HASHER_THEME:+HASHER_THEME="$HASHER_THEME"} \
    timeout 90 "$BIN" --screenshot "$OUT" "$@"
STATUS=$?
set -e

if [[ $STATUS -eq 124 ]]; then
    echo "Timeout (90 s) senza screenshot" >&2
    exit 124
fi
if [[ $STATUS -ne 0 ]]; then
    echo "hasher è terminato con codice $STATUS" >&2
    exit "$STATUS"
fi
if [[ ! -s "$OUT" ]]; then
    echo "Screenshot non creato: $OUT" >&2
    exit 1
fi

echo "Screenshot: $OUT"
