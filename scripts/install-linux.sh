#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BINARY_NAME="rfindfiles"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_APP_DIR="${HOME}/.local/share/applications"
INSTALL_BIN_PATH="${INSTALL_BIN_DIR}/${BINARY_NAME}"
DESKTOP_FILE_PATH="${INSTALL_APP_DIR}/${BINARY_NAME}.desktop"

mkdir -p "${INSTALL_BIN_DIR}"
mkdir -p "${INSTALL_APP_DIR}"

cd "${PROJECT_ROOT}"

echo "Building release binary..."
cargo build --release --bin "${BINARY_NAME}"

echo "Installing binary to ${INSTALL_BIN_PATH}"
cp "target/release/${BINARY_NAME}" "${INSTALL_BIN_PATH}"
chmod +x "${INSTALL_BIN_PATH}"

echo "Creating desktop entry at ${DESKTOP_FILE_PATH}"
cat > "${DESKTOP_FILE_PATH}" <<EOF
[Desktop Entry]
Type=Application
Name=File Finder
Comment=Simple GTK file finder written in Rust
Exec=${INSTALL_BIN_PATH}
Icon=system-file-manager
Terminal=false
Categories=Utility;FileTools;
StartupNotify=true
EOF

echo "Installation complete."
echo "You can launch the app from your application menu or run:"
echo "  ${INSTALL_BIN_PATH}"
