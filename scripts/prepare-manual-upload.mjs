import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFile, lstat, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, 'github-upload');
const maxFiles = 100;
const maxBytes = 25 * 1024 * 1024;
const prohibited = /^(?:\.git|\.local|\.upstream|_site|target|github-upload)(?:\/|$)|\.(?:recovery|secret)$/i;

function within(parent, child) {
  const path = relative(parent, child);
  return path !== '' && path !== '..' && !path.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`) && !resolve(child).startsWith('\\\\');
}

const rootReal = await realpath(root);
if (!within(rootReal, output)) throw new Error('Upload output must stay inside the repository');
try {
  const existing = await realpath(output);
  if (!within(rootReal, existing)) throw new Error('Existing upload output resolves outside the repository');
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
}

const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { cwd: root, encoding: 'utf8' })
  .split('\0').filter(Boolean).sort();
if (!files.length) throw new Error('Git found no files to upload');
if (files.some(path => prohibited.test(path))) throw new Error('Git candidate list contains a private or generated path');

const core = files.filter(path => !path.startsWith('evidence/'));
const evidence = files.filter(path => path.startsWith('evidence/'));
if (core.length > maxFiles) throw new Error(`Core has ${core.length} files; split it before browser upload`);
const batches = [{ name: '01-core', files: core }];
for (let i = 0; i < evidence.length; i += maxFiles) {
  batches.push({ name: `${String(batches.length + 1).padStart(2, '0')}-evidence`, files: evidence.slice(i, i + maxFiles) });
}

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
let copied = 0;
for (const batch of batches) {
  const base = join(output, batch.name);
  for (const path of batch.files) {
    const source = resolve(root, path);
    if (!within(rootReal, source)) throw new Error(`Invalid source path: ${path}`);
    const stat = await lstat(source);
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`Expected an ordinary file: ${path}`);
    if (stat.size > maxBytes) throw new Error(`Browser upload limit exceeded: ${path}`);
    const target = join(base, path);
    await mkdir(dirname(target), { recursive: true });
    await copyFile(source, target);
    const original = createHash('sha256').update(await readFile(source)).digest('hex');
    const duplicate = createHash('sha256').update(await readFile(target)).digest('hex');
    if (original !== duplicate) throw new Error(`Copy mismatch: ${path}`);
    copied++;
  }
}
if (copied !== files.length) throw new Error('Not every public file was copied');

const instructions = [
  'THRESHOLD — MANUAL GITHUB WEB UPLOAD',
  '',
  'Create a new PUBLIC repository on GitHub. Leave its README, license, and .gitignore options unchecked.',
  'For each numbered batch in order, open that batch folder in File Explorer, select its CONTENTS,',
  'and drag those contents onto GitHub’s “upload files” page at the repository root.',
  'Do not drag the numbered batch folder itself; the app/, crates/, docs/, evidence/, scripts/,',
  'and .github/ paths must land at the repository root. Commit each batch before the next.',
  'After batch 01, verify .github/workflows/pages.yml and README.md appear in the repository.',
  'After the last batch, set Settings → Pages → Build and deployment → Source to GitHub Actions.',
  'Run “Publish Threshold demo” in Actions if the initial push did not deploy after Pages setup.',
  'Do not upload the original Tari folder: it also contains private .local/ recovery material.',
  '',
  ...batches.map(batch => `${batch.name}: ${batch.files.length} files`),
  `Total: ${files.length} files`,
  '',
  'This folder is a local export and is ignored by Git. Re-run node scripts/prepare-manual-upload.mjs',
  'after changing source files so the batches stay current.',
];
await writeFile(join(output, 'UPLOAD-INSTRUCTIONS.txt'), `${instructions.join('\n')}\n`);
console.log(`Prepared ${files.length} public files in ${batches.length} browser-sized batches at ${output}`);
for (const batch of batches) console.log(`  ${batch.name}: ${batch.files.length} files`);
