import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer, request as proxyRequest } from 'node:http';
import { extname, join, normalize, resolve } from 'node:path';

const root = resolve(process.cwd(), 'src');
const port = Number(process.env.PORT || process.argv[2] || 4173);
const types = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml; charset=utf-8'
};

function proxyOpenCode(req, res, urlPath) {
  const targetPath = urlPath.replace(/^\/opencode/, '') || '/';
  const proxy = proxyRequest({
    hostname: '127.0.0.1',
    port: 4096,
    path: targetPath,
    method: req.method,
    headers: { ...req.headers, host: '127.0.0.1:4096' }
  }, (upstream) => {
    res.writeHead(upstream.statusCode || 502, upstream.headers);
    upstream.pipe(res);
  });
  proxy.on('error', () => {
    if (!res.headersSent) {
      res.writeHead(502, { 'Content-Type': 'application/json; charset=utf-8' });
      res.end(JSON.stringify({ error: 'OpenCode sidecar is unavailable on 127.0.0.1:4096.' }));
    }
  });
  req.pipe(proxy);
}

createServer((req, res) => {
  const urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
  if (urlPath === '/opencode' || urlPath.startsWith('/opencode/')) {
    proxyOpenCode(req, res, urlPath);
    return;
  }

  const requestPath = urlPath === '/' ? '/index.html' : urlPath;
  const filePath = normalize(join(root, requestPath));
  if (!filePath.startsWith(root) || !existsSync(filePath) || statSync(filePath).isDirectory()) {
    res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
    res.end('Not found');
    return;
  }

  res.writeHead(200, {
    'Content-Type': types[extname(filePath)] || 'application/octet-stream',
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff'
  });
  createReadStream(filePath).pipe(res);
}).listen(port, '0.0.0.0', () => {
  console.log(`KUBERA prototype running at http://0.0.0.0:${port}`);
  console.log(`OpenCode development proxy: http://0.0.0.0:${port}/opencode/* → 127.0.0.1:4096/*`);
});
