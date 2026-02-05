#!/usr/bin/env node
/**
 * Script pour générer les icônes de l'application avec fond dégradé
 * Part du logo.svg original
 */

const sharp = require('sharp');
const path = require('path');
const fs = require('fs');
const { execFileSync } = require('child_process');

const ICONS_DIR = path.join(__dirname, '../src-tauri/icons');
const LOGO_SVG = path.join(__dirname, '../src/assets/logo.svg');

// Couleurs du dégradé de l'interface
const GRADIENT_START = '#8B5CF6'; // --accent-primary violet
const GRADIENT_END = '#06B6D4';   // --accent-secondary cyan

async function createGradientIcon(size) {
  const cornerRadius = Math.round(size * 0.22);

  // 1. Créer le fond dégradé
  const bgSvg = `<svg width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="${GRADIENT_START}"/>
        <stop offset="100%" stop-color="${GRADIENT_END}"/>
      </linearGradient>
    </defs>
    <rect width="${size}" height="${size}" rx="${cornerRadius}" fill="url(#grad)"/>
  </svg>`;
  const background = await sharp(Buffer.from(bgSvg)).png().toBuffer();

  // 2. Rendre le logo en blanc avec une taille appropriée
  // Le logo original est 2816x1536, on le redimensionne pour tenir dans ~85% de l'icône
  const logoSize = Math.round(size * 0.85);
  const logoHeight = Math.round(logoSize * (1536 / 2816)); // Garder le ratio

  // Lire et modifier le SVG du logo pour le rendre blanc
  let logoSvgContent = fs.readFileSync(LOGO_SVG, 'utf8');
  logoSvgContent = logoSvgContent.replace(/fill="#000000"/g, 'fill="#FFFFFF"');

  // Rendre le SVG en PNG puis trim pour enlever l'espace vide
  const logoRaw = await sharp(Buffer.from(logoSvgContent), { density: 72 })
    .png()
    .toBuffer();

  // Trim l'espace transparent autour du logo
  const logoTrimmed = await sharp(logoRaw)
    .trim()
    .png()
    .toBuffer();

  // Redimensionner à la taille voulue
  const logo = await sharp(logoTrimmed)
    .resize(logoSize, logoHeight, { fit: 'inside', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toBuffer();

  // Obtenir les dimensions réelles du logo redimensionné
  const logoMeta = await sharp(logo).metadata();

  // 3. Composer le logo sur le fond (centré)
  const offsetX = Math.round((size - logoMeta.width) / 2);
  const offsetY = Math.round((size - logoMeta.height) / 2);

  return sharp(background)
    .composite([{ input: logo, top: offsetY, left: offsetX }])
    .png()
    .toBuffer();
}

async function generateIcons() {
  console.log('Generating app icons with gradient background from logo.svg...');

  const sizes = [
    { size: 32, name: '32x32.png' },
    { size: 128, name: '128x128.png' },
    { size: 256, name: '128x128@2x.png' },
    { size: 512, name: 'icon.png' },
  ];

  for (const { size, name } of sizes) {
    const buffer = await createGradientIcon(size);
    const outputPath = path.join(ICONS_DIR, name);
    await sharp(buffer).toFile(outputPath);
    console.log(`  Created: ${name} (${size}x${size})`);
  }

  // Créer le fichier .icns pour macOS
  console.log('Creating macOS .icns file...');
  const iconsetDir = path.join(ICONS_DIR, 'icon.iconset');
  if (!fs.existsSync(iconsetDir)) fs.mkdirSync(iconsetDir);

  const icnsSizes = [
    { size: 16, name: 'icon_16x16.png' },
    { size: 32, name: 'icon_16x16@2x.png' },
    { size: 32, name: 'icon_32x32.png' },
    { size: 64, name: 'icon_32x32@2x.png' },
    { size: 128, name: 'icon_128x128.png' },
    { size: 256, name: 'icon_128x128@2x.png' },
    { size: 256, name: 'icon_256x256.png' },
    { size: 512, name: 'icon_256x256@2x.png' },
    { size: 512, name: 'icon_512x512.png' },
    { size: 1024, name: 'icon_512x512@2x.png' },
  ];

  for (const { size, name } of icnsSizes) {
    const buffer = await createGradientIcon(size);
    await sharp(buffer).toFile(path.join(iconsetDir, name));
  }

  // Utiliser iconutil pour créer le .icns (macOS uniquement)
  try {
    execFileSync('iconutil', ['-c', 'icns', iconsetDir, '-o', path.join(ICONS_DIR, 'icon.icns')]);
    console.log('  Created: icon.icns');
  } catch (err) {
    console.log('  Skipped .icns (not on macOS or iconutil not found)');
  }

  // Nettoyer le dossier iconset
  fs.rmSync(iconsetDir, { recursive: true, force: true });

  console.log('Done!');
}

generateIcons().catch(console.error);
