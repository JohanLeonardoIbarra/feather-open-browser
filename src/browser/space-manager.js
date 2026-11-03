import { randomUUID } from 'node:crypto';
import { EventEmitter } from 'node:events';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { WebContentsView, session as electronSession } from 'electron';
import { normalizeNavigationTarget } from './navigation.js';
import { buildVisualActionScript, ALLOWED_KEYS } from './visual-agent.js';

const TOOLBAR_HEIGHT = 76;
const MAX_ACTIONS_PER_BATCH = 32;
const AGENT_WORLD_ID = 1937;
const NEW_TAB_FILE = fileURLToPath(new URL('../ui/newtab.html', import.meta.url));
const NEW_TAB_URL = pathToFileURL(NEW_TAB_FILE).href;

function snapshotInPage() {
  const refs = new Map();
  window.__openBrowserAgentRefs = refs;
  let nextRef = 1;
  const visible = (el) => {
    const rect = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden' && style.opacity !== '0';
  };
  const text = (el) => (el.getAttribute('aria-label') || el.innerText ||
    el.value || el.getAttribute('placeholder') || el.getAttribute('alt') ||
    el.getAttribute('title') || '').replace(/\s+/g, ' ').trim().slice(0, 240);
  const role = (el) => {
    if (el.getAttribute('role')) return el.getAttribute('role');
    const tag = el.tagName.toLowerCase();
    if (tag === 'a') return 'link';
    if (tag === 'button') return 'button';
    if (tag === 'textarea') return 'textbox';
    if (tag === 'select') return 'combobox';
    if (tag === 'input') {
      const type = (el.getAttribute('type') || 'text').toLowerCase();
      if (type === 'checkbox') return 'checkbox';
      if (type === 'radio') return 'radio';
      if (type === 'submit' || type === 'button') return 'button';
      return 'textbox';
    }
    return 'generic';
  };
  const candidates = Array.from(document.querySelectorAll(
    'a[href],button,input,textarea,select,[role],[contenteditable="true"]'
  )).filter(visible).slice(0, 180);
  const rows = [];
  for (const el of candidates) {
    const ref = 'e' + nextRef++;
    refs.set(ref, el);
    const name = text(el) || '(sin nombre)';
    const href = el instanceof HTMLAnchorElement ? ' -> ' + el.href : '';
    const value = (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)
      ? ' value=' + JSON.stringify(el.value.slice(0, 100)) : '';
    rows.push(role(el) + ' ' + JSON.stringify(name) + ' [ref=' + ref + ']' + href + value);
  }
  const textBlocks = Array.from(document.querySelectorAll('h1,h2,h3,p,li'))
    .filter(visible).slice(0, 100)
    .map((el) => el.tagName.toLowerCase() + ': ' + (el.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 280))
    .filter((line) => line.length > 4);
  return {
    title: document.title,
    url: location.href,
    elements: rows,
    text: textBlocks,
    truncated: candidates.length >= 180
  };
}

const SNAPSHOT_SCRIPT = `(${snapshotInPage.toString()})()`;

export class SpaceManager extends EventEmitter {
  constructor(mainWindow) {
    super();
    this.mainWindow = mainWindow;
    this.spaces = new Map();
    this.activeSpaceId = null;
  }

  async initialize() {
    this.spaces.clear();
    this.activeSpaceId = null;
    this.emitState();
  }

  async createSpace({ name = 'Agent space', activate = true } = {}) {
    const id = randomUUID();
    const partition = `agent-${id}`;
    const browserSession = electronSession.fromPartition(partition);
    const space = {
      id,
      name: String(name).trim().slice(0, 48) || 'Agent space',
      kind: 'agent',
      partition,
      session: browserSession,
      tabs: new Map(),
      activeTabId: null,
      busyCount: 0
    };
    this.spaces.set(id, space);
    this.setupNetworkTracing(space);
    this.emitState();
    if (activate || !this.activeSpaceId) await this.activateSpace(id);
    return { id, name: space.name, kind: space.kind };
  }

  findTabByWebContentsId(webContentsId) {
    for (const space of this.spaces.values()) {
      for (const tab of space.tabs.values()) {
        if (tab.view?.webContents?.id === webContentsId) {
          return tab;
        }
      }
    }
    return null;
  }

  setupNetworkTracing(space) {
    const webRequest = space.session.webRequest;
    webRequest.onBeforeRequest((details, callback) => {
      try {
        if (details.webContentsId) {
          const tab = this.findTabByWebContentsId(details.webContentsId);
          if (tab) {
            tab.networkRequests.push({
              id: details.id,
              url: details.url,
              method: details.method,
              resourceType: details.resourceType,
              timestamp: Date.now(),
              status: 'pending'
            });
            if (tab.networkRequests.length > 200) tab.networkRequests.shift();
          }
        }
      } catch {}
      callback({});
    });

    webRequest.onCompleted((details) => {
      try {
        if (details.webContentsId) {
          const tab = this.findTabByWebContentsId(details.webContentsId);
          if (tab) {
            const req = tab.networkRequests.find((r) => r.id === details.id);
            if (req) {
              req.status = details.statusCode;
              req.fromCache = details.fromCache;
              req.completedAt = Date.now();
            } else {
              tab.networkRequests.push({
                id: details.id,
                url: details.url,
                method: details.method,
                resourceType: details.resourceType,
                status: details.statusCode,
                fromCache: details.fromCache,
                timestamp: Date.now()
              });
              if (tab.networkRequests.length > 200) tab.networkRequests.shift();
            }
          }
        }
      } catch {}
    });

    webRequest.onErrorOccurred((details) => {
      try {
        if (details.webContentsId) {
          const tab = this.findTabByWebContentsId(details.webContentsId);
          if (tab) {
            const req = tab.networkRequests.find((r) => r.id === details.id);
            if (req) {
              req.status = 'error';
              req.error = details.error;
              req.completedAt = Date.now();
            } else {
              tab.networkRequests.push({
                id: details.id,
                url: details.url,
                method: details.method,
                resourceType: details.resourceType,
                status: 'error',
                error: details.error,
                timestamp: Date.now()
              });
              if (tab.networkRequests.length > 200) tab.networkRequests.shift();
            }
          }
        }
      } catch {}
    });
  }

  getSpace(spaceId) {
    const space = this.spaces.get(spaceId);
    if (!space) throw new Error('No existe ese Space.');
    return space;
  }

  getTab(spaceId, tabId) {
    const space = this.getSpace(spaceId);
    const tab = space.tabs.get(tabId);
    if (!tab) throw new Error('No existe esa pestaña.');
    return { space, tab };
  }

  async createTab(spaceId, target = 'about:blank', { activate = true } = {}) {
    const space = this.getSpace(spaceId);
    const id = randomUUID();
    const view = new WebContentsView({
      webPreferences: {
        session: space.session,
        contextIsolation: true,
        sandbox: true,
        nodeIntegration: false,
        webSecurity: true
      }
    });
    const tab = {
      id,
      view,
      title: 'Nueva pestaña',
      url: 'about:blank',
      loading: false,
      internalNewTab: false,
      visible: false,
      consoleLogs: [],
      networkRequests: []
    };
    space.tabs.set(id, tab);
    this.mainWindow.contentView.addChildView(view);
    view.setVisible(false);
    this.bindTabEvents(space, tab);
    view.webContents.setWindowOpenHandler(({ url }) => {
      try {
        const safeUrl = normalizeNavigationTarget(url);
        void this.createTab(space.id, safeUrl, { activate: true }).catch(() => {});
      } catch {
        // Pages cannot use popups to open local files or executable protocols.
      }
      return { action: 'deny' };
    });

    if (activate || !space.activeTabId) space.activeTabId = id;
    if (activate || !this.activeSpaceId || this.activeSpaceId === spaceId) {
      await this.activateSpace(spaceId);
    }
    this.emitState();
    await this.navigate(spaceId, id, target);
    this.layout();
    this.emitState();
    return this.serializeTab(tab);
  }

  bindTabEvents(space, tab) {
    const wc = tab.view.webContents;
    const changed = () => {
      tab.internalNewTab = wc.getURL() === NEW_TAB_URL;
      tab.title = tab.internalNewTab ? 'Nueva pestaña' : (wc.getTitle() || tab.title);
      tab.url = tab.internalNewTab ? 'about:blank' : (wc.getURL() || tab.url);
      this.emitState();
    };
    wc.on('did-start-loading', () => {
      tab.loading = true;
      this.emitState();
    });
    wc.on('did-stop-loading', () => {
      tab.loading = false;
      changed();
    });
    wc.on('page-title-updated', (_event, title) => {
      tab.title = tab.internalNewTab ? 'Nueva pestaña' : (title || tab.title);
      this.emitState();
    });
    wc.on('did-navigate', changed);
    wc.on('did-navigate-in-page', changed);
    wc.on('console-message', (_event, level, message, line, sourceId) => {
      const levels = ['debug', 'info', 'warning', 'error'];
      tab.consoleLogs.push({
        level: levels[level] || 'info',
        message,
        line,
        sourceId,
        timestamp: Date.now()
      });
      if (tab.consoleLogs.length > 200) tab.consoleLogs.shift();
    });
    wc.on('render-process-gone', () => {
      tab.loading = false;
      tab.title = 'La pestaña dejó de responder';
      this.emitState();
    });
  }

  async activateSpace(spaceId) {
    const space = this.getSpace(spaceId);
    if (!space.activeTabId && space.tabs.size) space.activeTabId = space.tabs.keys().next().value;
    this.activeSpaceId = spaceId;
    for (const currentSpace of this.spaces.values()) {
      for (const tab of currentSpace.tabs.values()) {
        const isVisible = currentSpace.id === spaceId && tab.id === currentSpace.activeTabId;
        tab.view.setVisible(isVisible);
        tab.visible = isVisible;
      }
    }
    this.layout();
    this.emitState();
  }

  activateTab(spaceId, tabId) {
    const { space } = this.getTab(spaceId, tabId);
    space.activeTabId = tabId;
    return this.activateSpace(spaceId);
  }

  async navigate(spaceId, tabId, target, { allowFile = true } = {}) {
    const { tab } = this.getTab(spaceId, tabId);
    const url = normalizeNavigationTarget(target, { allowFile });
    if (url === 'about:blank') {
      tab.internalNewTab = true;
      await tab.view.webContents.loadFile(NEW_TAB_FILE);
    } else {
      tab.internalNewTab = false;
      await tab.view.webContents.loadURL(url);
    }
    return this.serializeTab(tab);
  }

  async goBack(spaceId, tabId) {
    const { tab } = this.getTab(spaceId, tabId);
    if (tab.view.webContents.navigationHistory.canGoBack()) tab.view.webContents.navigationHistory.goBack();
  }

  async goForward(spaceId, tabId) {
    const { tab } = this.getTab(spaceId, tabId);
    if (tab.view.webContents.navigationHistory.canGoForward()) tab.view.webContents.navigationHistory.goForward();
  }

  reload(spaceId, tabId) {
    const { tab } = this.getTab(spaceId, tabId);
    tab.view.webContents.reload();
  }

  async closeTab(spaceId, tabId) {
    const space = this.getSpace(spaceId);
    const tab = space.tabs.get(tabId);
    if (!tab) return;
    if (space.tabs.size === 1) {
      await this.navigate(spaceId, tabId, 'about:blank');
      return;
    }
    this.mainWindow.contentView.removeChildView(tab.view);
    tab.view.webContents.close();
    space.tabs.delete(tabId);
    if (space.activeTabId === tabId) space.activeTabId = space.tabs.keys().next().value;
    await this.activateSpace(this.activeSpaceId || spaceId);
    this.emitState();
  }

  async closeSpace(spaceId) {
    const space = this.spaces.get(spaceId);
    if (!space) return;
    for (const tab of space.tabs.values()) {
      this.mainWindow.contentView.removeChildView(tab.view);
      tab.view.webContents.close();
    }
    await space.session.clearStorageData();
    await space.session.clearCache();
    this.spaces.delete(spaceId);
    if (this.activeSpaceId === spaceId) {
      this.activeSpaceId = this.spaces.keys().next().value || null;
      if (this.activeSpaceId) {
        await this.activateSpace(this.activeSpaceId);
      } else {
        this.layout();
      }
    }
    this.emitState();
  }

  async snapshot(spaceId, tabId) {
    const { tab } = this.getTab(spaceId, tabId);
    const wc = tab.view.webContents;
    const result = await wc.executeJavaScriptInIsolatedWorld(AGENT_WORLD_ID, [{ code: SNAPSHOT_SCRIPT }]);
    const errorLogs = tab.consoleLogs.filter((l) => l.level === 'error');
    return {
      ...result,
      url: tab.internalNewTab ? 'about:blank' : wc.getURL(),
      title: tab.internalNewTab ? 'Nueva pestaña' : wc.getTitle(),
      consoleErrors: errorLogs.slice(-5),
      networkRequestsCount: tab.networkRequests.length,
      note: 'MVP: el snapshot cubre el documento principal; el recorrido de iframes está pendiente.'
    };
  }

  async screenshot(spaceId, tabId) {
    const { tab } = this.getTab(spaceId, tabId);
    const wc = tab.view.webContents;
    const image = await wc.capturePage();
    const size = image.getSize();
    const buffer = image.toPNG();
    const base64 = buffer.toString('base64');
    return {
      width: size.width,
      height: size.height,
      mimeType: 'image/png',
      dataUrl: `data:image/png;base64,${base64}`,
      base64
    };
  }

  async getHtml(spaceId, tabId, ref = null) {
    const { tab } = this.getTab(spaceId, tabId);
    const wc = tab.view.webContents;
    const code = ref
      ? `(() => {
          const el = window.__openBrowserAgentRefs?.get(${JSON.stringify(ref)});
          if (!el?.isConnected) return null;
          return el.outerHTML;
        })()`
      : `(() => {
          return document.documentElement ? document.documentElement.outerHTML : '';
        })()`;
    const html = await wc.executeJavaScriptInIsolatedWorld(AGENT_WORLD_ID, [{ code }]);
    if (ref && html === null) throw new Error('Ref no encontrada o elemento desconectado.');
    return {
      url: tab.internalNewTab ? 'about:blank' : wc.getURL(),
      title: tab.internalNewTab ? 'Nueva pestaña' : wc.getTitle(),
      ref: ref || null,
      html: html || ''
    };
  }

  getConsoleLogs(spaceId, tabId, limit = 100) {
    const { tab } = this.getTab(spaceId, tabId);
    const max = Math.max(1, Math.min(Number(limit) || 100, 200));
    return {
      logs: tab.consoleLogs.slice(-max)
    };
  }

  getNetworkRequests(spaceId, tabId, limit = 100) {
    const { tab } = this.getTab(spaceId, tabId);
    const max = Math.max(1, Math.min(Number(limit) || 100, 200));
    return {
      requests: tab.networkRequests.slice(-max)
    };
  }

  async runActions(spaceId, tabId, actions) {
    if (!Array.isArray(actions) || actions.length === 0 || actions.length > MAX_ACTIONS_PER_BATCH) {
      throw new Error(`Envía entre 1 y ${MAX_ACTIONS_PER_BATCH} acciones.`);
    }
    const { space } = this.getTab(spaceId, tabId);
    space.busyCount += 1;
    this.emitState();
    try {
      const results = [];
      let step = 1;
      for (const action of actions) {
        results.push(await this.runAction(spaceId, tabId, action, { step, total: actions.length }));
        step++;
      }
      return { results, snapshot: await this.snapshot(spaceId, tabId) };
    } finally {
      space.busyCount = Math.max(0, space.busyCount - 1);
      this.emitState();
    }
  }

  async runAction(spaceId, tabId, action, { step = 1, total = 1 } = {}) {
    if (!action || typeof action !== 'object') throw new Error('Acción no válida.');
    if (action.type === 'navigate') return this.navigate(spaceId, tabId, action.url, { allowFile: false });
    if (action.type === 'wait') {
      const ms = Math.max(0, Math.min(Number(action.ms) || 0, 5000));
      await new Promise((resolve) => setTimeout(resolve, ms));
      return { waited: ms };
    }
    if (action.type === 'back') {
      await this.goBack(spaceId, tabId);
      return { ok: true };
    }
    if (action.type === 'forward') {
      await this.goForward(spaceId, tabId);
      return { ok: true };
    }
    if (action.type === 'reload') {
      this.reload(spaceId, tabId);
      return { ok: true };
    }
    if (action.type === 'press') {
      if (!ALLOWED_KEYS.has(action.key)) throw new Error('Tecla no permitida.');
      const { tab } = this.getTab(spaceId, tabId);
      const isTabVisible = Boolean(tab.visible && this.activeSpaceId === spaceId);
      const fast = Boolean(process.env.OPEN_BROWSER_FAST_ACTIONS === '1' || !isTabVisible);
      const code = buildVisualActionScript(action, { fast, step, total });
      const result = await tab.view.webContents.executeJavaScriptInIsolatedWorld(AGENT_WORLD_ID, [{ code }]);
      if (!result?.ok) throw new Error(result?.error || 'No hay un elemento enfocado para enviar la tecla.');
      return { ok: true };
    }
    if (action.type === 'click' || action.type === 'fill') {
      if (typeof action.ref !== 'string' || !/^e\d+$/.test(action.ref)) throw new Error('Ref no válida; toma un snapshot nuevo.');
      const { tab } = this.getTab(spaceId, tabId);
      const isTabVisible = Boolean(tab.visible && this.activeSpaceId === spaceId);
      const fast = Boolean(process.env.OPEN_BROWSER_FAST_ACTIONS === '1' || !isTabVisible);
      const code = buildVisualActionScript(action, { fast, step, total });
      const result = await tab.view.webContents.executeJavaScriptInIsolatedWorld(AGENT_WORLD_ID, [{ code }]);
      if (!result?.ok) throw new Error(result?.error || 'El elemento ya no está disponible; toma un snapshot nuevo.');
      return { ok: true };
    }
    throw new Error(`Tipo de acción no soportado: ${String(action.type)}`);
  }

  layout() {
    const { width, height } = this.mainWindow.getContentBounds();
    for (const space of this.spaces.values()) {
      for (const tab of space.tabs.values()) {
        const isVisible = Boolean(this.activeSpaceId && space.id === this.activeSpaceId && tab.id === space.activeTabId);
        tab.view.setVisible(isVisible);
        tab.visible = isVisible;
        if (isVisible) {
          tab.view.setBounds({
            x: 0,
            y: TOOLBAR_HEIGHT,
            width,
            height: Math.max(0, height - TOOLBAR_HEIGHT)
          });
        }
      }
    }
  }

  serializeTab(tab) {
    const wc = tab.view.webContents;
    return {
      id: tab.id,
      title: tab.internalNewTab ? 'Nueva pestaña' : (wc.getTitle() || tab.title),
      url: tab.internalNewTab ? 'about:blank' : (wc.getURL() || tab.url),
      loading: tab.loading,
      canGoBack: wc.navigationHistory.canGoBack(),
      canGoForward: wc.navigationHistory.canGoForward()
    };
  }

  getState() {
    return {
      activeSpaceId: this.activeSpaceId,
      spaces: [...this.spaces.values()].map((space) => ({
        id: space.id,
        name: space.name,
        kind: 'agent',
        status: space.busyCount > 0
          ? 'working'
          : space.tabs.get(space.activeTabId)?.loading
            ? 'loading'
            : 'ready',
        activeTabId: space.activeTabId,
        tabs: [...space.tabs.values()].map((tab) => this.serializeTab(tab))
      }))
    };
  }

  emitState() {
    this.emit('state', this.getState());
  }
}
