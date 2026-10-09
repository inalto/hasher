#!/usr/bin/env bash
# Crea dist/Hasher.app e dist/hasher-<versione>-macos-universal.zip (solo macOS).
#
# Uso:   scripts/bundle-macos.sh <versione> <binario>
# Esempio (dopo aver costruito e unito con lipo i binari x86_64 e aarch64):
#   scripts/bundle-macos.sh 0.1.0 target/universal/hasher
#
# L'app NON è firmata né notarizzata: alla prima apertura Gatekeeper la blocca. Vedi il README:
#   xattr -dr com.apple.quarantine Hasher.app
# La firma "ad-hoc" applicata qui (se codesign è disponibile) serve solo a far girare il binario
# su Apple Silicon; non sostituisce una firma con certificato Developer ID.
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Uso: $0 <versione> <binario>" >&2
    exit 2
fi

VERSION="${1#v}"
BIN="$2"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
APP="$DIST/Hasher.app"
ZIP="$DIST/hasher-${VERSION}-macos-universal.zip"
# CFBundleVersion / CFBundleShortVersionString: solo la parte numerica (senza -rc1, +build...).
PLIST_VERSION="${VERSION%%[-+]*}"

if [[ ! -f "$BIN" ]]; then
    echo "Binario non trovato: $BIN" >&2
    exit 1
fi
if ! command -v ditto >/dev/null 2>&1; then
    echo "ditto non trovato: questo script funziona solo su macOS." >&2
    exit 1
fi

rm -rf "${APP:?}" "${ZIP:?}"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/LICENSES-fonts"

install -m 755 "$BIN" "$APP/Contents/MacOS/hasher"
bash "$ROOT/scripts/make-icns.sh" "$APP/Contents/Resources/icon.icns"
install -m 644 "$ROOT"/assets/fonts/LICENSE-*.txt "$APP/Contents/Resources/LICENSES-fonts/"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Hasher</string>
    <key>CFBundleDisplayName</key>
    <string>Hasher</string>
    <key>CFBundleIdentifier</key>
    <string>net.martini-multimedia.hasher</string>
    <key>CFBundleVersion</key>
    <string>${PLIST_VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${PLIST_VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>hasher</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>© Martini Multimedia s.a.s.</string>
</dict>
</plist>
PLIST

if command -v plutil >/dev/null 2>&1; then
    plutil -lint "$APP/Contents/Info.plist"
fi

# Firma ad-hoc (nessuna identità): necessaria perché il binario giri su Apple Silicon.
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$APP"
fi

ditto -c -k --keepParent "$APP" "$ZIP"

echo "Creato: $APP"
echo "Creato: $ZIP"
