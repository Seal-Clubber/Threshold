import { mkdir, readFile, writeFile, copyFile, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, '_site');
const app = join(root, 'app');

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
const html = (await readFile(join(app, 'wire.html'), 'utf8'))
  .replace('href="/wire.css"', 'href="./wire.css"')
  .replace('src="/wire.js"', 'src="./wire.js"')
  .replace('src="/sandbox.js"', 'src="./sandbox.js"');
await writeFile(join(output, 'index.html'), html);
await copyFile(join(app, 'wire.css'), join(output, 'wire.css'));
await copyFile(join(app, 'wire.js'), join(output, 'wire.js'));
await copyFile(join(app, 'sandbox.js'), join(output, 'sandbox.js'));
await writeFile(join(output, '.nojekyll'), '');
console.log(`Built the single-page bounty map at ${output}`);
