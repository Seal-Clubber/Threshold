import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, dirname, relative, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '_site');
const base = '/threshold/'; // Emulates a GitHub project URL, including its subpath.
const port = Number(process.env.THRESHOLD_PAGES_PORT || 4766);
const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.json': 'application/json' };

http.createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://localhost').pathname;
  if (request.method !== 'GET' || !pathname.startsWith(base)) {
    response.writeHead(404).end('Not found');
    return;
  }
  try {
    const decoded = decodeURIComponent(pathname.slice(base.length));
    const target = resolve(root, decoded.endsWith('/') || !decoded ? join(decoded, 'index.html') : decoded);
    const within = relative(root, target);
    if (within.startsWith('..') || within.includes(':')) throw new Error('Invalid path');
    const body = await readFile(target);
    response.writeHead(200, { 'Content-Type': `${types[extname(target)] || 'application/octet-stream'}; charset=utf-8`, 'Cache-Control': 'no-store' }).end(body);
  } catch {
    response.writeHead(404).end('Not found');
  }
}).listen(port, '127.0.0.1', () => console.log(`Static Pages preview: http://127.0.0.1:${port}${base}`));
