#!/usr/bin/env node
/**
 * Post-build script to prepend TypeScript type definitions to index.d.ts
 *
 * napi-rs generates index.d.ts with class declarations but doesn't know about
 * our custom interfaces. This script prepends types.d.ts content to the
 * generated index.d.ts to provide complete type definitions.
 */

const fs = require('fs');
const path = require('path');

const rootDir = path.resolve(__dirname, '..');
const typesPath = path.join(rootDir, 'types.d.ts');
const indexDtsPath = path.join(rootDir, 'index.d.ts');

// Read files
const typesContent = fs.readFileSync(typesPath, 'utf-8');
const indexDtsContent = fs.readFileSync(indexDtsPath, 'utf-8');

// Check if already prepended (avoid double prepending)
if (indexDtsContent.includes('// ============================================================================')) {
  console.log('index.d.ts already contains type definitions, skipping prepend');
  process.exit(0);
}

// Prepend types to index.d.ts
const combinedContent = typesContent + '\n\n' + indexDtsContent;
fs.writeFileSync(indexDtsPath, combinedContent);

console.log('Successfully prepended types.d.ts to index.d.ts');
console.log(`Total lines: ${combinedContent.split('\n').length}`);
