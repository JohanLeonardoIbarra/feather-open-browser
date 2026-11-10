// Tauri <-> Feather Open Browser UI Bridge
// Bridges Tauri v2 IPC commands and events to the standard window.browser interface

if (!window.browser && window.__TAURI__) {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  window.browser = {
    getState: () => invoke('get_state'),
    createTab: (spaceId, url) => invoke('create_tab', { spaceId, url }),
    activateTab: (spaceId, tabId) => invoke('activate_tab', { spaceId, tabId }),
    closeTab: (spaceId, tabId) => invoke('close_tab', { spaceId, tabId }),
    navigate: (spaceId, tabId, url) => invoke('navigate', { spaceId, tabId, url }),
    goBack: (spaceId, tabId) => invoke('go_back', { spaceId, tabId }),
    goForward: (spaceId, tabId) => invoke('go_forward', { spaceId, tabId }),
    reload: (spaceId, tabId) => invoke('reload', { spaceId, tabId }),
    createSpace: (name, _inheritCookies) => invoke('create_space', { name }),
    activateSpace: (spaceId) => invoke('activate_space', { spaceId }),
    closeSpace: (spaceId) => invoke('close_space', { spaceId }),
    getApiInfo: () => invoke('get_api_info'),
    onState: (callback) => {
      let unlistenFn = null;
      listen('browser:state', (event) => callback(event.payload)).then((unlisten) => {
        unlistenFn = unlisten;
      });
      return () => {
        if (unlistenFn) unlistenFn();
      };
    }
  };
}
