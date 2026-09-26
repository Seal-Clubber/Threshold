import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const appDir = dirname(fileURLToPath(import.meta.url));
const port = Number(process.env.THRESHOLD_PORT || 4765);
const files = new Map([
  ['/', ['wire.html', 'text/html; charset=utf-8']],
  ['/wire.css', ['wire.css', 'text/css; charset=utf-8']],
  ['/wire.js', ['wire.js', 'text/javascript; charset=utf-8']],
  ['/sandbox.js', ['sandbox.js', 'text/javascript; charset=utf-8']],
]);

function respond(response, status, body, contentType = 'text/plain; charset=utf-8') {
  response.writeHead(status, {
    'Content-Type': contentType,
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff',
    'Content-Security-Policy': "default-src 'self'; style-src 'self'; script-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'",
  });
  response.end(body);
}

http.createServer(async (request, response) => {
  if (request.method !== 'GET') return respond(response, 405, 'Method not allowed');
  const path = new URL(request.url, 'http://localhost').pathname;
  const entry = files.get(path);
  if (!entry) return respond(response, 404, 'Not found');
  try {
    return respond(response, 200, await readFile(resolve(appDir, entry[0])), entry[1]);
  } catch {
    return respond(response, 503, 'Demo asset unavailable');
  }
}).listen(port, '127.0.0.1', () => {
  console.log(`Threshold demo: http://127.0.0.1:${port}/`);
});
