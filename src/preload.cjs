const { contextBridge, ipcRenderer } = require('electron');

const invoke = (channel, ...args) => ipcRenderer.invoke(channel, ...args);

contextBridge.exposeInMainWorld('browser', {
  getState: () => invoke('browser:get-state'),
  createTab: (spaceId, url) => invoke('browser:create-tab', spaceId, url),
  activateTab: (spaceId, tabId) => invoke('browser:activate-tab', spaceId, tabId),
  closeTab: (spaceId, tabId) => invoke('browser:close-tab', spaceId, tabId),
  navigate: (spaceId, tabId, url) => invoke('browser:navigate', spaceId, tabId, url),
  goBack: (spaceId, tabId) => invoke('browser:back', spaceId, tabId),
  goForward: (spaceId, tabId) => invoke('browser:forward', spaceId, tabId),
  reload: (spaceId, tabId) => invoke('browser:reload', spaceId, tabId),
  createSpace: (name, inheritCookies) => invoke('browser:create-space', name, inheritCookies),
  activateSpace: (spaceId) => invoke('browser:activate-space', spaceId),
  closeSpace: (spaceId) => invoke('browser:close-space', spaceId),
  getApiInfo: () => invoke('browser:api-info'),
  onState: (callback) => {
    const listener = (_event, state) => callback(state);
    ipcRenderer.on('browser:state', listener);
    return () => ipcRenderer.removeListener('browser:state', listener);
  }
});
