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

test('agent API routes verification endpoints: screenshot, html, snapshot, extract, network, console', async () => {
  const userDataPath = await mkdtemp(join(tmpdir(), 'open-browser-verify-test-'));
  const manager = {
    getState: () => ({ spaces: [] }),
    screenshot: async (sId, tId) => ({ width: 800, height: 600, mimeType: 'image/png', base64: 'abc', dataUrl: 'data:image/png;base64,abc' }),
    getHtml: async (sId, tId, ref, options = {}) => {
      if (ref === 'e999') throw new Error("El ref 'e999' expiró o el elemento ya no está en el DOM. Vuelve a tomar un browser_snapshot.");
      return {
        url: 'https://example.com',
        title: 'Example',
        ref,
        selector: options.selector || null,
        textOnly: Boolean(options.textOnly),
        totalChars: 32,
        offset: options.offset || 0,
        maxChars: options.maxChars || 20000,
        truncated: false,
        html: options.textOnly ? 'Hello' : '<html><body>Hello</body></html>'
      };
    },
    snapshot: async (sId, tId, options = {}) => ({
      url: 'https://example.com',
      title: 'Example',
      elements: ['button "Ordenar Fecha" [ref=e1]'],
      totalElements: 1,
      offset: options.offset || 0,
      limit: options.limit || 180,
      truncated: false,
      tables: [
        {
          headers: ['Fecha [ref=e1]', 'Estado'],
          rows: [['2026-11-14', 'Activo']],
          totalRows: 1,
          rowOffset: options.tableRowOffset || 0,
          truncated: false
        }
      ],
      text: ['h1: Portal']
    }),
    extract: async (sId, tId, options = {}) => ({
      url: 'https://example.com',
      title: 'Example',
      selector: options.selector,
      totalMatches: 1,
      offset: options.offset || 0,
      limit: options.limit || 50,
      truncated: false,
      items: [{ index: 0, tag: 'tr', cells: ['2026-11-14', 'Activo'] }]
    }),
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

    const pageHtml = await client.html('s1', 't1', { selector: 'main', maxChars: 5000, textOnly: true });
    assert.equal(pageHtml.title, 'Example');
    assert.equal(pageHtml.selector, 'main');
    assert.equal(pageHtml.textOnly, true);
    assert.equal(pageHtml.html, 'Hello');

    await assert.rejects(
      () => client.html('s1', 't1', 'e999'),
      /El ref 'e999' expiró/
    );

    const snap = await client.snapshot('s1', 't1', { limit: 50, maxTableRows: 25 });
    assert.equal(snap.tables.length, 1);
    assert.deepEqual(snap.tables[0].headers, ['Fecha [ref=e1]', 'Estado']);
    assert.deepEqual(snap.tables[0].rows[0], ['2026-11-14', 'Activo']);

    const extracted = await client.extract('s1', 't1', { selector: 'tbody tr', limit: 10 });
    assert.equal(extracted.selector, 'tbody tr');
    assert.equal(extracted.items.length, 1);
    assert.deepEqual(extracted.items[0].cells, ['2026-11-14', 'Activo']);

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
