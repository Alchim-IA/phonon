# Build WakaScribe pour Windows
# Exécuter sur une machine Windows avec PowerShell

$ErrorActionPreference = "Stop"

Write-Host "=== Build WakaScribe pour Windows ===" -ForegroundColor Cyan

# Vérifier Node.js
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Node.js non trouvé. Téléchargez-le sur: https://nodejs.org/" -ForegroundColor Red
    exit 1
}

# Vérifier Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust non trouvé. Téléchargez-le sur: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Aller à la racine du projet
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location (Join-Path $scriptPath "..")

# Installer les dépendances npm
Write-Host "Installation des dépendances npm..." -ForegroundColor Yellow
npm install

# Build en mode release
Write-Host "Build de l'application..." -ForegroundColor Yellow
npm run tauri build

Write-Host ""
Write-Host "=== Build terminé ===" -ForegroundColor Green
Write-Host "L'application se trouve dans: src-tauri\target\release\bundle\"
Write-Host "  - MSI: src-tauri\target\release\bundle\msi\"
Write-Host "  - NSIS: src-tauri\target\release\bundle\nsis\"
