import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

function defaultConfigPath() {
  if (process.env.OPEN_BROWSER_CONFIG) return process.env.OPEN_BROWSER_CONFIG;
  if (process.platform === 'win32') {
    return join(process.env.APPDATA || join(homedir(), 'AppData', 'Roaming'), 'open-browser', 'agent-api.json');
  }
  if (process.platform === 'darwin') {
    return join(homedir(), 'Library', 'Application Support', 'open-browser', 'agent-api.json');
  }
  return join(process.env.XDG_CONFIG_HOME || join(homedir(), '.config'), 'open-browser', 'agent-api.json');
}

export class OpenBrowserClient {
  constructor(config) {
    this.baseUrl = `http://${config.host}:${config.port}`;
    this.token = config.token;
  }

  static async connect({ configPath = defaultConfigPath() } = {}) {
    let config;
    try {
      config = JSON.parse(await readFile(configPath, 'utf8'));
    } catch {
      throw new Error(`No se pudo leer la configuración de Open Browser: ${configPath}. ¿Está abierta la aplicación?`);
    }
    return new OpenBrowserClient(config);
  }

  async request(path, { method = 'GET', body } = {}) {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers: {
        authorization: `Bearer ${this.token}`,
        ...(body === undefined ? {} : { 'content-type': 'application/json' })
      },
      signal: AbortSignal.timeout(30_000),
      ...(body === undefined ? {} : { body: JSON.stringify(body) })
    });
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || `Open Browser respondió HTTP ${response.status}.`);
    return result;
  }

  async spaces() {
    return (await this.request('/v1/spaces')).spaces;
  }

  async createSpace({ name, url } = {}) {
    return this.request('/v1/spaces', { method: 'POST', body: { name, url } });
  }

  async closeSpace(spaceId) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}`, { method: 'DELETE' });
  }

  async tabs(spaceId) {
    return (await this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs`)).tabs;
  }

  async createTab(spaceId, url) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs`, {
      method: 'POST', body: { url }
    });
  }

  async closeTab(spaceId, tabId) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}`, {
      method: 'DELETE'
    });
  }

  async navigate(spaceId, tabId, url) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/navigate`, {
      method: 'POST', body: { url }
    });
  }

  async snapshot(spaceId, tabId) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/snapshot`);
  }

  async screenshot(spaceId, tabId) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/screenshot`);
  }

  async html(spaceId, tabId, ref = null) {
    const query = ref ? `?ref=${encodeURIComponent(ref)}` : '';
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/html${query}`);
  }

  async network(spaceId, tabId, limit = null) {
    const query = limit ? `?limit=${encodeURIComponent(limit)}` : '';
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/network${query}`);
  }

  async consoleLogs(spaceId, tabId, limit = null) {
    const query = limit ? `?limit=${encodeURIComponent(limit)}` : '';
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/console${query}`);
  }

  async actions(spaceId, tabId, actions) {
    return this.request(`/v1/spaces/${encodeURIComponent(spaceId)}/tabs/${encodeURIComponent(tabId)}/actions`, {
      method: 'POST', body: { actions }
    });
  }
}
