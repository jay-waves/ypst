import { build } from 'esbuild';
import { copyFile, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
await build({ entryPoints: [resolve(dir, 'intl.mjs')], outfile: resolve(dir, 'intl.js'), bundle: true, format: 'iife', platform: 'neutral', target: 'es2020', minifyWhitespace: true, minifyIdentifiers: true, minifySyntax: false, banner: { js: 'globalThis.Intl ??= {};' }, legalComments: 'inline' });
await copyFile(resolve(dir, '../node_modules/highlight.js/LICENSE'), resolve(dir, 'highlight.js-LICENSE'));
await copyFile(resolve(dirname(require.resolve('linkedom/package.json')), 'LICENSE'), resolve(dir, 'LinkeDOM-LICENSE'));
await build({ entryPoints: [resolve(dir, 'highlight.mjs')], outfile: resolve(dir, 'highlight.js'), bundle: true, format: 'iife', platform: 'neutral', mainFields: ['module', 'main'], target: 'es2020', minify: true, legalComments: 'inline' });

// Keep the checked-in standalone renderer and its license in sync with npm.
await copyFile(require.resolve('pele/global'), resolve(dir, '../html/pele.global.js'));
await copyFile(resolve(dirname(require.resolve('pele/package.json')), 'LICENSE'), resolve(dir, '../html/LICENSE-Pele.txt'));

const formatjsPackages = ['intl-numberformat', 'intl-locale', 'intl-getcanonicallocales', 'intl-pluralrules', 'intl-localematcher', 'bigdecimal'];
const licenses = await Promise.all(formatjsPackages.map(async name => `@formatjs/${name}\n${await readFile(resolve(dir, '../node_modules/@formatjs', name, 'LICENSE.md'), 'utf8')}`));
await writeFile(resolve(dir, 'FormatJS-LICENSE'), licenses.join('\n\n'));
const peleVersion = JSON.parse(await readFile(resolve(dir, '../node_modules/pele/package.json'), 'utf8')).version;
const peleHash = createHash('sha256').update(await readFile(resolve(dir, '../html/pele.global.js'))).digest('hex');
const manifestPath = resolve(dir, '../typst.toml');
const manifest = await readFile(manifestPath, 'utf8');
const updated = manifest.replace(/(\[tool\.ypst-template\.assets\.pele\][\s\S]*?)(?=\n\[|$)/, section => section.replace(/version = "[^"]+"/, `version = "${peleVersion}"`).replace(/pele@[^/]+/, `pele@${peleVersion}`).replace(/sha256 = "[^"]+"/, `sha256 = "${peleHash}"`));
if (updated === manifest && !manifest.includes(`sha256 = "${peleHash}"`)) throw new Error('Pele asset manifest section not found');
await writeFile(manifestPath, updated);
