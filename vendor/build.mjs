import { build } from 'esbuild';
import * as chartist from 'chartist';
import * as server from 'chartist/server';
import { copyFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const moduleFile = (name, value, global) => writeFile(resolve(dir, name),
  Object.keys(value).map(key => `export const ${key}=globalThis.${global}.${key};`).join('\n'));
await copyFile(resolve(dir, '../node_modules/highlight.js/LICENSE'), resolve(dir, 'highlight.js-LICENSE'));
await build({ entryPoints: [resolve(dir, 'renderer.mjs')], outfile: resolve(dir, 'renderer.js'), bundle: true,
  format: 'iife', platform: 'neutral', mainFields: ['module', 'main'], target: 'es2020',
  minifyWhitespace: true, minifyIdentifiers: true, minifySyntax: false, legalComments: 'inline' });
await moduleFile('chartist-module.js', chartist, '__vizChartist');
await moduleFile('chartist-server-module.js', server, '__vizChartistServer');
const chartistDir = dirname(require.resolve('chartist/package.json'));
const linkedomDir = dirname(require.resolve('linkedom/package.json'));
await copyFile(resolve(chartistDir, 'dist/index.css'), resolve(dir, 'chartist.css'));
await copyFile(resolve(chartistDir, 'LICENSE-MIT'), resolve(dir, 'Chartist-LICENSE'));
await copyFile(resolve(linkedomDir, 'LICENSE'), resolve(dir, 'LinkeDOM-LICENSE'));

await build({ entryPoints: [resolve(dir, 'highlight.mjs')], outfile: resolve(dir, 'highlight.js'), bundle: true, format: 'iife', platform: 'neutral', mainFields: ['module', 'main'], target: 'es2020', minify: true, legalComments: 'inline' });
