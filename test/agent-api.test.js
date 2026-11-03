import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AgentApiServer } from '../src/agent-api/server.js';
import { OpenBrowserClient } from '../src/agent-api/client.js';

test('agent API binds a loopback port and requires its local bearer token', async () => {
  const userDataPath = await mkdtemp(join(tmpdir(), 'open-browser-test-'));
  const manager = { getState: () => ({ spaces: [] }) };
  const api = new AgentApiServer({ manager, userDataPath });

  try {
    const info = await api.start();
    assert.equal(info.host, '127.0.0.1');
    const config = JSON.parse(await readFile(info.configPath, 'utf8'));
    assert.equal(config.port, info.port);
    assert.equal((await stat(info.configPath)).mode & 0o077, 0);

    const base = `http://${info.host}:${info.port}`;
    const health = await fetch(`${base}/health`);
    assert.equal(health.status, 200);

    const unauthorized = await fetch(`${base}/v1/spaces`);
    assert.equal(unauthorized.status, 401);

    const fromWebPage = await fetch(`${base}/v1/spaces`, {
      headers: { authorization: `Bearer ${config.token}`, origin: 'https://example.com' }
    });
    assert.equal(fromWebPage.status, 403);

    const authorized = await fetch(`${base}/v1/spaces`, {
      headers: { authorization: `Bearer ${config.token}` }
    });
    assert.equal(authorized.status, 200);
    assert.deepEqual(await authorized.json(), { spaces: [] });

    const client = new OpenBrowserClient(config);
    assert.deepEqual(await client.spaces(), []);
  } finally {
    await api.stop();
    await rm(userDataPath, { recursive: true, force: true });
  }
});

test('agent API routes verification endpoints: screenshot, html, network, console', async () => {
  const userDataPath = await mkdtemp(join(tmpdir(), 'open-browser-verify-test-'));
  const manager = {
    getState: () => ({ spaces: [] }),
    screenshot: async (sId, tId) => ({ width: 800, height: 600, mimeType: 'image/png', base64: 'abc', dataUrl: 'data:image/png;base64,abc' }),
    getHtml: async (sId, tId, ref) => ({ url: 'https://example.com', title: 'Example', ref, html: '<html><body>Hello</body></html>' }),
    getNetworkRequests: (sId, tId, limit) => ({ requests: [{ url: 'https://example.com/api', status: 200 }] }),
    getConsoleLogs: (sId, tId, limit) => ({ logs: [{ level: 'info', message: 'Ready' }] })
  };
  const api = new AgentApiServer({ manager, userDataPath });

  try {
    const info = await api.start();
    const config = JSON.parse(await readFile(info.configPath, 'utf8'));
    const client = new OpenBrowserClient(config);

    const shot = await client.screenshot('s1', 't1');
    assert.equal(shot.width, 800);
    assert.equal(shot.base64, 'abc');

    const pageHtml = await client.html('s1', 't1');
    assert.equal(pageHtml.title, 'Example');
    assert.match(pageHtml.html, /Hello/);

    const net = await client.network('s1', 't1');
    assert.equal(net.requests.length, 1);
    assert.equal(net.requests[0].status, 200);

    const logs = await client.consoleLogs('s1', 't1');
    assert.equal(logs.logs.length, 1);
    assert.equal(logs.logs[0].message, 'Ready');
  } finally {
    await api.stop();
    await rm(userDataPath, { recursive: true, force: true });
  }
});
