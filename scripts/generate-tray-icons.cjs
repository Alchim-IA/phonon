#!/usr/bin/env node
/**
 * Génère les icônes du tray (blanches pour le mode sombre macOS)
 */

const sharp = require('sharp');
const path = require('path');
const fs = require('fs');

const ICONS_DIR = path.join(__dirname, '../src-tauri/icons');
const LOGO_SVG = path.join(__dirname, '../src/assets/logo.svg');

async function createTrayIcon(size) {
  // Lire et modifier le SVG du logo pour le rendre blanc
  let logoSvgContent = fs.readFileSync(LOGO_SVG, 'utf8');
  logoSvgContent = logoSvgContent.replace(/fill="#000000"/g, 'fill="#FFFFFF"');

  // Rendre le SVG puis trim l'espace vide
  const logoRaw = await sharp(Buffer.from(logoSvgContent), { density: 72 })
    .png()
    .toBuffer();

  const logoTrimmed = await sharp(logoRaw)
    .trim()
    .png()
    .toBuffer();

  // Redimensionner pour remplir la taille avec une petite marge
  const targetSize = Math.round(size * 0.9);
  return sharp(logoTrimmed)
    .resize(targetSize, targetSize, { fit: 'inside', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .extend({
      top: Math.round((size - targetSize) / 2),
      bottom: Math.round((size - targetSize) / 2),
      left: Math.round((size - targetSize) / 2),
      right: Math.round((size - targetSize) / 2),
      background: { r: 0, g: 0, b: 0, alpha: 0 }
    })
    .png()
    .toBuffer();
}

async function generateTrayIcons() {
  console.log('Generating tray icons...');

  // Taille standard pour les tray icons macOS
  const tray22 = await createTrayIcon(22);
  await sharp(tray22).toFile(path.join(ICONS_DIR, 'tray-iconTemplate.png'));
  console.log('  Created: tray-iconTemplate.png (22x22)');

  const tray44 = await createTrayIcon(44);
  await sharp(tray44).toFile(path.join(ICONS_DIR, 'tray-iconTemplate@2x.png'));
  console.log('  Created: tray-iconTemplate@2x.png (44x44)');

  console.log('Done!');
}

generateTrayIcons().catch(console.error);
