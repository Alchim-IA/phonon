# Guide de Build WakaScribe

## Prérequis communs

### Node.js (v18+)
- macOS: `brew install node`
- Windows: https://nodejs.org/

### Rust
- macOS/Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Windows: https://rustup.rs/

---

## macOS Intel (x86_64)

### Sur votre Mac Intel:

```bash
# Cloner le projet
git clone <repo-url>
cd scribe

# Exécuter le script de build
./scripts/build-macos-intel.sh
```

### Ou manuellement:

```bash
npm install
npm run tauri build -- --target x86_64-apple-darwin
```

### Résultat:
- `src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/` → Fichier .dmg
- `src-tauri/target/x86_64-apple-darwin/release/bundle/macos/` → Application .app

---

## macOS Apple Silicon (M1/M2/M3/M4)

### Sur votre Mac M1+:

```bash
# Cloner le projet
git clone <repo-url>
cd scribe

# Exécuter le script de build
./scripts/build-macos-arm.sh
```

### Ou manuellement:

```bash
npm install
npm run tauri build -- --target aarch64-apple-darwin
```

### Résultat:
- `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/` → Fichier .dmg
- `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/` → Application .app

---

## Windows

### Sur votre PC Windows:

```powershell
# Cloner le projet
git clone <repo-url>
cd scribe

# Exécuter le script de build (PowerShell)
.\scripts\build-windows.ps1
```

### Ou manuellement:

```powershell
npm install
npm run tauri build
```

### Résultat:
- `src-tauri\target\release\bundle\msi\` → Installeur MSI
- `src-tauri\target\release\bundle\nsis\` → Installeur NSIS (.exe)

---

## Universal Binary macOS (optionnel)

Pour créer un binaire universel qui fonctionne sur Intel ET Apple Silicon depuis un Mac:

```bash
# Ajouter les deux targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Build universel
npm run tauri build -- --target universal-apple-darwin
```

---

## Résumé des commandes

| Plateforme | Commande |
|------------|----------|
| macOS Intel | `npm run tauri build -- --target x86_64-apple-darwin` |
| macOS M1+ | `npm run tauri build -- --target aarch64-apple-darwin` |
| macOS Universal | `npm run tauri build -- --target universal-apple-darwin` |
| Windows | `npm run tauri build` |
| Linux | `npm run tauri build` |

---

## Dépendances système

### Linux (pour le build)
```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# Fedora
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel

# Arch
sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 librsvg
```

### Linux (pour l'auto-paste)
```bash
# X11
sudo apt install xclip xdotool

# Wayland
sudo apt install wl-clipboard wtype
```
