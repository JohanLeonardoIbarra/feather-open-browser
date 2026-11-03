import { app, BrowserWindow, ipcMain } from 'electron';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SpaceManager } from './browser/space-manager.js';
import { AgentApiServer } from './agent-api/server.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
let mainWindow;
let manager;
let agentApi;
let apiInfo;

app.setName('Open Browser');
const userDataPath = process.env.OPEN_BROWSER_USER_DATA || join(app.getPath('appData'), 'open-browser');
mkdirSync(userDataPath, { recursive: true, mode: 0o700 });
app.setPath('userData', userDataPath);

function registerIpc() {
  ipcMain.handle('browser:get-state', () => manager.getState());
  ipcMain.handle('browser:create-tab', (_event, spaceId, url) => manager.createTab(spaceId, url || 'about:blank'));
  ipcMain.handle('browser:activate-tab', (_event, spaceId, tabId) => manager.activateTab(spaceId, tabId));
  ipcMain.handle('browser:close-tab', (_event, spaceId, tabId) => manager.closeTab(spaceId, tabId));
  ipcMain.handle('browser:navigate', (_event, spaceId, tabId, url) => manager.navigate(spaceId, tabId, url));
  ipcMain.handle('browser:back', (_event, spaceId, tabId) => manager.goBack(spaceId, tabId));
  ipcMain.handle('browser:forward', (_event, spaceId, tabId) => manager.goForward(spaceId, tabId));
  ipcMain.handle('browser:reload', (_event, spaceId, tabId) => manager.reload(spaceId, tabId));
  ipcMain.handle('browser:create-space', async (_event, name) => {
    const space = await manager.createSpace({ name, kind: 'agent', activate: true });
    await manager.createTab(space.id, 'about:blank');
    return space;
  });
  ipcMain.handle('browser:activate-space', (_event, spaceId) => manager.activateSpace(spaceId));
  ipcMain.handle('browser:close-space', (_event, spaceId) => manager.closeSpace(spaceId));
  ipcMain.handle('browser:api-info', () => apiInfo);
}

async function createWindow() {
  const testWindowSize = process.env.OPEN_BROWSER_WINDOW_SIZE?.match(/^(\d+)x(\d+)$/);
  mainWindow = new BrowserWindow({
    width: testWindowSize ? Number(testWindowSize[1]) : 1380,
    height: testWindowSize ? Number(testWindowSize[2]) : 900,
    minWidth: 800,
    minHeight: 580,
    show: false,
    title: 'Open Browser',
    backgroundColor: '#101318',
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false
    }
  });

  manager = new SpaceManager(mainWindow);
  manager.on('state', (state) => {
    if (!mainWindow.isDestroyed()) mainWindow.webContents.send('browser:state', state);
  });
  await manager.initialize();
  if (process.env.OPEN_BROWSER_DEMO_AGENT_SCREEN === '1') {
    const demoSpace = await manager.createSpace({ name: 'Research · demo', kind: 'agent', activate: false });
    await manager.createTab(demoSpace.id, 'about:blank', { activate: false });
    manager.getSpace(demoSpace.id).busyCount = 1;
    await manager.activateSpace(demoSpace.id);
  }
  agentApi = new AgentApiServer({ manager, userDataPath: app.getPath('userData') });
  apiInfo = await agentApi.start();
  registerIpc();
  mainWindow.on('resize', () => manager.layout());
  mainWindow.on('closed', () => { mainWindow = null; });
  await mainWindow.loadFile(join(__dirname, 'ui/index.html'));
  const uiReady = await mainWindow.webContents.executeJavaScript('window.__openBrowserUiReady === true');
  if (!uiReady) throw new Error('La interfaz del navegador no terminó de inicializarse.');
  mainWindow.show();
  if (process.env.OPEN_BROWSER_SCREENSHOT) {
    await mainWindow.webContents.executeJavaScript('document.fonts.ready.then(() => true)');
    await mainWindow.webContents.executeJavaScript('new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
    const image = await mainWindow.capturePage();
    writeFileSync(process.env.OPEN_BROWSER_SCREENSHOT, image.toPNG(), { mode: 0o600 });
    if (manager.activeSpaceId) {
      const activeSpace = manager.getSpace(manager.activeSpaceId);
      const activeTab = manager.getTab(activeSpace.id, activeSpace.activeTabId).tab;
      await activeTab.view.webContents.executeJavaScript('document.fonts.ready.then(() => true)');
      const pageImage = await activeTab.view.webContents.capturePage();
      const pagePath = process.env.OPEN_BROWSER_SCREENSHOT.replace(/\.png$/i, '-webview.png');
      writeFileSync(pagePath, pageImage.toPNG(), { mode: 0o600 });
    }
  }
}

app.whenReady().then(async () => {
  await createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) void createWindow();
  });
}).catch((error) => {
  console.error('No se pudo iniciar Open Browser:', error);
  app.quit();
});

app.on('before-quit', () => {
  void agentApi?.stop();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
