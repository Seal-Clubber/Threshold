import { readFile, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const output = resolve(dirname(fileURLToPath(import.meta.url)), '..', '_site');
const files = (await readdir(output)).sort();
const expected = ['.nojekyll', 'index.html', 'sandbox.js', 'wire.css', 'wire.js'];
if (JSON.stringify(files) !== JSON.stringify(expected)) throw new Error(`Unexpected Pages files: ${files.join(', ')}`);
const html = await readFile(join(output, 'index.html'), 'utf8');
if (!html.includes('id="sandbox-dialog"') || !html.includes('id="flow"')) throw new Error('Map or sandbox missing');
if (/(?:href|src)="\//.test(html)) throw new Error('Root-relative URL breaks project Pages paths');
if (/\/explore|\/showcase|\/observer|\/api\//.test(html)) throw new Error('Extra page or API reference remains');
const script = await readFile(join(output, 'wire.js'), 'utf8');
if (/\bfetch\s*\(/.test(script)) throw new Error('Static map should not query a backend');
const sandbox = await readFile(join(output, 'sandbox.js'), 'utf8');
if (/\bfetch\s*\(/.test(sandbox) || /\bchoices\b|\bplan\b/.test(sandbox)) throw new Error('Sandbox must remain independent of the map and backend');
console.log('Checked independent sandbox, single-page map and project-path assets');
