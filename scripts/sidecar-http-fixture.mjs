#!/usr/bin/env node
/**
 * Sidecar HTTP fixture — independent of MCP process model (T10 / L22-VF).
 * Starts a minimal localhost Spark HTTP mock for /api/* only.
 * Does not spawn Node MCP or Host MCP.
 */
import http from 'node:http';
import { createServer } from 'node:net';

export async function ephemeralPort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
    server.on('error', reject);
  });
}

function respondJson(res, status, body) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

/**
 * @param {number} port
 * @returns {Promise<http.Server>}
 */
export function startSidecarHttpFixture(port) {
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url || '/', `http://127.0.0.1:${port}`);

    if (req.method === 'GET' && url.pathname === '/api/status') {
      respondJson(res, 200, { ok: true, fixture: 'sidecar-http' });
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/todo-tasks') {
      respondJson(res, 200, []);
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/notes-catalogs') {
      respondJson(res, 200, { items: [] });
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/notes-latest-digests') {
      respondJson(res, 200, { items: [] });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/notes-search') {
      respondJson(res, 200, { items: [] });
      return;
    }

    respondJson(res, 404, { error: 'not found' });
  });

  return new Promise((resolve, reject) => {
    server.listen(port, '127.0.0.1', () => resolve(server));
    server.on('error', reject);
  });
}

export function stopSidecarHttpFixture(server) {
  return new Promise((resolve) => {
    server.close(() => resolve());
  });
}

/** Self-check: fixture starts/stops without any MCP process. */
async function main() {
  if (!process.argv.includes('--self-test')) return;
  const port = await ephemeralPort();
  const server = await startSidecarHttpFixture(port);
  const res = await fetch(`http://127.0.0.1:${port}/api/status`);
  if (!res.ok) {
    await stopSidecarHttpFixture(server);
    throw new Error(`sidecar fixture /api/status failed: ${res.status}`);
  }
  await stopSidecarHttpFixture(server);
  console.log('sidecar-http-fixture self-test PASSED');
}

main().catch((err) => {
  console.error('sidecar-http-fixture FAILED:', err);
  process.exit(1);
});
