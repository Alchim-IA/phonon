#!/bin/bash
# Build WakaScribe pour macOS Intel (x86_64)
# Exécuter sur une machine Mac Intel

set -e

echo "=== Build WakaScribe pour macOS Intel ==="

# Vérifier qu'on est sur macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo "Erreur: Ce script doit être exécuté sur macOS"
    exit 1
fi

# Installer les dépendances si nécessaire
if ! command -v node &> /dev/null; then
    echo "Node.js non trouvé. Installez-le via: brew install node"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Rust non trouvé. Installez-le via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Aller à la racine du projet
cd "$(dirname "$0")/.."

# Installer les dépendances npm
echo "Installation des dépendances npm..."
npm install

# Build en mode release
echo "Build de l'application..."
npm run tauri build -- --target x86_64-apple-darwin

echo ""
echo "=== Build terminé ==="
echo "L'application se trouve dans: src-tauri/target/x86_64-apple-darwin/release/bundle/"
echo "  - DMG: src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/"
echo "  - APP: src-tauri/target/x86_64-apple-darwin/release/bundle/macos/"
