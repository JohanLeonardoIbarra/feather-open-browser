import { createServer } from 'node:http';
import { randomBytes, timingSafeEqual } from 'node:crypto';
import { chmod, mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { normalizeNavigationTarget } from '../browser/navigation.js';

const MAX_BODY_BYTES = 1024 * 1024;

function sendJson(response, status, value) {
  response.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'x-content-type-options': 'nosniff'
  });
  response.end(JSON.stringify(value));
}

async function readJson(request) {
  const chunks = [];
  let bytes = 0;
  for await (const chunk of request) {
    bytes += chunk.length;
    if (bytes > MAX_BODY_BYTES) throw Object.assign(new Error('Body demasiado grande.'), { status: 413 });
    chunks.push(chunk);
  }
  if (!bytes) return {};
  try {
    return JSON.parse(Buffer.concat(chunks).toString('utf8'));
  } catch {
    throw Object.assign(new Error('JSON no válido.'), { status: 400 });
  }
}

function matchesToken(request, token) {
  const supplied = request.headers.authorization?.replace(/^Bearer\s+/i, '') ?? '';
  const expectedBuffer = Buffer.from(token);
  const suppliedBuffer = Buffer.from(supplied);
  return expectedBuffer.length === suppliedBuffer.length && timingSafeEqual(expectedBuffer, suppliedBuffer);
}

export class AgentApiServer {
  constructor({ manager, userDataPath }) {
    this.manager = manager;
    this.userDataPath = userDataPath;
    this.token = randomBytes(32).toString('hex');
    this.server = null;
    this.port = null;
    this.configPath = join(userDataPath, 'agent-api.json');
  }

  async start() {
    await mkdir(this.userDataPath, { recursive: true });
    this.server = createServer((request, response) => {
      void this.handle(request, response);
    });
    await new Promise((resolve, reject) => {
      this.server.once('error', reject);
      this.server.listen(0, '127.0.0.1', resolve);
    });
    this.port = this.server.address().port;
    await writeFile(this.configPath, JSON.stringify({
      host: '127.0.0.1',
      port: this.port,
      token: this.token,
      protocol: 'open-browser-agent-api-v1'
    }, null, 2), { mode: 0o600 });
    await chmod(this.configPath, 0o600);
    return { host: '127.0.0.1', port: this.port, configPath: this.configPath };
  }

  async stop() {
    if (this.server?.listening) {
      await new Promise((resolve) => this.server.close(resolve));
    }
    await rm(this.configPath, { force: true });
  }

  async handle(request, response) {
    try {
      const origin = request.headers.origin;
      if (origin) return sendJson(response, 403, { error: 'Las solicitudes desde páginas web no están permitidas.' });
      const url = new URL(request.url, 'http://127.0.0.1');
      if (request.method === 'GET' && url.pathname === '/health') {
        return sendJson(response, 200, { ok: true, service: 'open-browser-agent-api-v1' });
      }
      if (!matchesToken(request, this.token)) return sendJson(response, 401, { error: 'Token ausente o incorrecto.' });

      if (request.method === 'GET' && url.pathname === '/v1/spaces') {
        return sendJson(response, 200, { spaces: this.manager.getState().spaces });
      }
      if (request.method === 'POST' && url.pathname === '/v1/spaces') {
        const body = await readJson(request);
        const space = await this.manager.createSpace({ name: body.name || 'Agent space', activate: true });
        const target = body.url ? normalizeNavigationTarget(body.url) : 'about:blank';
        const tab = await this.manager.createTab(space.id, target, { activate: true });
        return sendJson(response, 201, { space, tab });
      }

      const spaceMatch = url.pathname.match(/^\/v1\/spaces\/([^/]+)$/);
      if (spaceMatch && request.method === 'DELETE') {
        await this.manager.closeSpace(decodeURIComponent(spaceMatch[1]));
        return sendJson(response, 200, { closed: true });
      }

      const tabsMatch = url.pathname.match(/^\/v1\/spaces\/([^/]+)\/tabs$/);
      if (tabsMatch && request.method === 'GET') {
        const space = this.manager.getSpace(decodeURIComponent(tabsMatch[1]));
        return sendJson(response, 200, {
          tabs: [...space.tabs.values()].map((tab) => this.manager.serializeTab(tab))
        });
      }
      if (tabsMatch && request.method === 'POST') {
        const body = await readJson(request);
        const target = body.url ? normalizeNavigationTarget(body.url) : 'about:blank';
        const tab = await this.manager.createTab(decodeURIComponent(tabsMatch[1]), target, { activate: true });
        return sendJson(response, 201, { tab });
      }

      const tabMatch = url.pathname.match(/^\/v1\/spaces\/([^/]+)\/tabs\/([^/]+)(?:\/(snapshot|navigate|actions|screenshot|html|network|console))?$/);
      if (tabMatch) {
        const spaceId = decodeURIComponent(tabMatch[1]);
        const tabId = decodeURIComponent(tabMatch[2]);
        const operation = tabMatch[3];
        if (request.method === 'DELETE' && !operation) {
          await this.manager.closeTab(spaceId, tabId);
          return sendJson(response, 200, { closed: true });
        }
        if (request.method === 'GET' && operation === 'snapshot') {
          return sendJson(response, 200, await this.manager.snapshot(spaceId, tabId));
        }
        if (request.method === 'GET' && operation === 'screenshot') {
          return sendJson(response, 200, await this.manager.screenshot(spaceId, tabId));
        }
        if (request.method === 'GET' && operation === 'html') {
          const ref = url.searchParams.get('ref');
          return sendJson(response, 200, await this.manager.getHtml(spaceId, tabId, ref));
        }
        if (request.method === 'GET' && operation === 'network') {
          const limit = url.searchParams.get('limit');
          return sendJson(response, 200, this.manager.getNetworkRequests(spaceId, tabId, limit));
        }
        if (request.method === 'GET' && operation === 'console') {
          const limit = url.searchParams.get('limit');
          return sendJson(response, 200, this.manager.getConsoleLogs(spaceId, tabId, limit));
        }
        if (request.method === 'POST' && operation === 'navigate') {
          const body = await readJson(request);
          return sendJson(response, 200, { tab: await this.manager.navigate(spaceId, tabId, body.url, { allowFile: false }) });
        }
        if (request.method === 'POST' && operation === 'actions') {
          const body = await readJson(request);
          return sendJson(response, 200, await this.manager.runActions(spaceId, tabId, body.actions));
        }
      }

      return sendJson(response, 404, { error: 'Ruta no encontrada.' });
    } catch (error) {
      const status = Number(error.status) || (error.message?.includes('No existe') ? 404 : 400);
      return sendJson(response, status, { error: error.message || 'Error interno.' });
    }
  }
}
