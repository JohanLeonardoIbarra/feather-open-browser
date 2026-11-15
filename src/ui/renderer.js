const spacesElement = document.querySelector('#spaces');
const tabsElement = document.querySelector('#tabs');
const tabsBar = document.querySelector('#tabs-bar');
const urlText = document.querySelector('#url-text');
const apiStatus = document.querySelector('#api-status');
const agentIndicator = document.querySelector('#agent-indicator');
const agentIndicatorLabel = document.querySelector('#agent-indicator-label');
const standbyView = document.querySelector('#standby-view');
const reloadButton = document.querySelector('#reload');

let state = null;

const iconShapes = {
  page: [['path', { d: 'M5 2.75h7l4 4v10.5H5z' }], ['path', { d: 'M12 2.75v4h4M8 11h5M8 14h5' }]],
  close: [['path', { d: 'm5 5 10 10M15 5 5 15' }]]
};

function icon(name, className = '') {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 20 20');
  svg.setAttribute('aria-hidden', 'true');
  if (className) svg.setAttribute('class', className);
  for (const [tag, attributes] of iconShapes[name] || []) {
    const child = document.createElementNS('http://www.w3.org/2000/svg', tag);
    for (const [key, value] of Object.entries(attributes)) child.setAttribute(key, value);
    svg.append(child);
  }
  return svg;
}

function activeSpace() {
  return state?.spaces.find((space) => space.id === state.activeSpaceId);
}

function activeTab() {
  const space = activeSpace();
  return space?.tabs.find((tab) => tab.id === space.activeTabId);
}

function renderSpaces() {
  spacesElement.replaceChildren();
  for (const space of state.spaces) {
    const chip = document.createElement('button');
    chip.type = 'button';
    chip.className = `space-chip${space.id === state.activeSpaceId ? ' active' : ''}`;
    chip.setAttribute('role', 'tab');
    chip.setAttribute('aria-selected', String(space.id === state.activeSpaceId));
    chip.dataset.status = space.status;
    chip.addEventListener('click', () => { void window.browser.activateSpace(space.id); });

    const dot = document.createElement('span');
    dot.className = 'space-dot';

    const label = document.createElement('span');
    label.className = 'space-label';
    label.textContent = space.name;

    const closeBtn = document.createElement('span');
    closeBtn.className = 'space-close-hint';
    closeBtn.title = 'Cerrar Space';
    closeBtn.append(icon('close'));
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      void window.browser.closeSpace(space.id);
    });

    chip.append(dot, label, closeBtn);
    spacesElement.append(chip);
  }
}

function renderTabs(space) {
  tabsElement.replaceChildren();
  if (!space) return;
  for (const tab of space.tabs) {
    const item = document.createElement('div');
    item.className = `tab${tab.id === space.activeTabId ? ' active' : ''}`;

    const select = document.createElement('button');
    select.type = 'button';
    select.className = 'tab-select';
    select.setAttribute('role', 'tab');
    select.setAttribute('aria-selected', String(tab.id === space.activeTabId));
    select.addEventListener('click', () => { void window.browser.activateTab(space.id, tab.id); });
    select.append(icon('page', 'tab-icon'));

    const title = document.createElement('span');
    title.className = 'tab-title';
    title.textContent = tab.title || 'Nueva pestaña';
    title.title = tab.url;
    select.append(title);

    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'tab-close';
    close.setAttribute('aria-label', 'Cerrar pestaña');
    close.append(icon('close'));
    close.addEventListener('click', () => { void window.browser.closeTab(space.id, tab.id); });

    item.append(select, close);
    tabsElement.append(item);
  }
}

function render() {
  if (!state) return;

  const space = activeSpace();
  const currentTab = activeTab();
  const hasSpaces = state.spaces.length > 0;
  const hasActiveSpace = Boolean(hasSpaces && space);

  if (standbyView) {
    standbyView.hidden = hasActiveSpace;
    standbyView.style.display = hasActiveSpace ? 'none' : 'flex';
  }
  if (hasActiveSpace) {
    document.documentElement.classList.add('has-active-space');
    document.body.classList.add('has-active-space');
  } else {
    document.documentElement.classList.remove('has-active-space');
    document.body.classList.remove('has-active-space');
  }
  if (tabsBar) {
    tabsBar.style.visibility = hasActiveSpace ? 'visible' : 'hidden';
  }

  renderSpaces();
  renderTabs(space);

  if (agentIndicator && agentIndicatorLabel) {
    if (!hasSpaces) {
      agentIndicator.className = 'agent-indicator standby';
      agentIndicatorLabel.textContent = 'En espera';
    } else if (space?.status === 'working') {
      agentIndicator.className = 'agent-indicator working';
      agentIndicatorLabel.textContent = 'Agente ejecutando';
    } else {
      agentIndicator.className = 'agent-indicator ready';
      agentIndicatorLabel.textContent = `${state.spaces.length} ${state.spaces.length === 1 ? 'Space activo' : 'Spaces activos'}`;
    }
  }

  if (urlText) {
    urlText.textContent = currentTab?.url || (hasSpaces ? 'about:blank' : '');
    urlText.title = currentTab?.url || '';
  }

  if (reloadButton) {
    reloadButton.disabled = !currentTab;
  }
}

if (reloadButton) {
  reloadButton.addEventListener('click', () => {
    const space = activeSpace();
    const tab = activeTab();
    if (space && tab) void window.browser.reload(space.id, tab.id);
  });
}

window.addEventListener('keydown', (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'r') {
    event.preventDefault();
    const space = activeSpace();
    const tab = activeTab();
    if (space && tab) void window.browser.reload(space.id, tab.id);
  }
});

window.browser.onState((nextState) => {
  state = nextState;
  render();
});

const [initialState, initialApiInfo] = await Promise.all([
  window.browser.getState(),
  window.browser.getApiInfo()
]);
state = initialState;
if (apiStatus && initialApiInfo) {
  apiStatus.title = `Servidor local en 127.0.0.1:${initialApiInfo.port}`;
}
render();
window.__openBrowserUiReady = true;

// ── Configuración del agente (MCP + Skill) ────────────────────────────────────
const settingsButton = document.querySelector('#settings');
const settingsModal = document.querySelector('#settings-modal');
const settingsTabs = document.querySelectorAll('.settings-tab');
const settingsPanes = document.querySelectorAll('.settings-pane');
const mcpConfigElement = document.querySelector('#mcp-config');
const skillTextElement = document.querySelector('#skill-text');
const apiHintElement = document.querySelector('#api-hint');
const copyConfigButton = document.querySelector('#copy-config');
const copySkillButton = document.querySelector('#copy-skill');

let skillTextPromise = null;

function buildMcpConfig(apiInfo) {
  if (!apiInfo) {
    mcpConfigElement.textContent = 'La API local no está disponible en este momento.';
    if (apiHintElement) apiHintElement.textContent = '';
    return null;
  }
  const config = {
    mcpServers: {
      'feather-open-browser': {
        type: 'http',
        url: `http://127.0.0.1:${apiInfo.port}/mcp`,
        headers: { Authorization: `Bearer ${apiInfo.token}` }
      }
    }
  };
  const text = JSON.stringify(config, null, 2);
  mcpConfigElement.textContent = text;
  if (apiHintElement) apiHintElement.textContent = `http://127.0.0.1:${apiInfo.port}/mcp`;
  return text;
}

function selectSettingsTab(name) {
  for (const tab of settingsTabs) {
    const active = tab.dataset.settingsTab === name;
    tab.classList.toggle('active', active);
    tab.setAttribute('aria-selected', String(active));
  }
  for (const pane of settingsPanes) {
    pane.hidden = pane.dataset.settingsPane !== name;
  }
}

function openSettings() {
  if (!settingsModal) return;
  settingsModal.hidden = false;
  if (mcpConfigElement && !mcpConfigElement.textContent) {
    buildMcpConfig(initialApiInfo);
  }
  if (skillTextElement && !skillTextElement.textContent) {
    loadSkillText();
  }
  const closeButton = settingsModal.querySelector('.settings-close');
  if (closeButton) closeButton.focus();
}

function loadSkillText() {
  if (!skillTextElement || skillTextPromise) return;
  skillTextPromise = window.browser.getSkillText();
  skillTextPromise
    .then((text) => {
      skillTextElement.textContent = text;
    })
    .catch(() => {
      skillTextElement.textContent = 'No se pudo cargar el texto de la skill.';
      skillTextPromise = null;
    });
}

function closeSettings() {
  if (settingsModal) settingsModal.hidden = true;
}

async function copyText(button, text, originalLabel) {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const helper = document.createElement('textarea');
    helper.value = text;
    helper.style.position = 'fixed';
    helper.style.opacity = '0';
    document.body.append(helper);
    helper.select();
    document.execCommand('copy');
    helper.remove();
  }
  button.textContent = '¡Copiado!';
  button.classList.add('copied');
  setTimeout(() => {
    button.textContent = originalLabel;
    button.classList.remove('copied');
  }, 1600);
}

if (settingsButton) {
  settingsButton.addEventListener('click', openSettings);
}
for (const tab of settingsTabs) {
  tab.addEventListener('click', () => selectSettingsTab(tab.dataset.settingsTab));
}
if (copyConfigButton) {
  copyConfigButton.addEventListener('click', () => {
    void copyText(copyConfigButton, mcpConfigElement?.textContent, 'Copiar JSON');
  });
}
if (copySkillButton) {
  copySkillButton.addEventListener('click', async () => {
    if (!skillTextElement.textContent) {
      loadSkillText();
      await skillTextPromise;
    }
    void copyText(copySkillButton, skillTextElement.textContent, 'Copiar skill');
  });
}
for (const element of settingsModal?.querySelectorAll('[data-settings-close]') || []) {
  element.addEventListener('click', closeSettings);
}
window.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && settingsModal && !settingsModal.hidden) closeSettings();
});

/* Controles de ventana (la ventana no tiene decoraciones nativas) */
function currentWindow() {
  const tauriWindow = window.__TAURI__?.window;
  return tauriWindow?.getCurrentWindow?.() ?? tauriWindow?.getCurrent?.();
}
if (/Mac|iPod|iPhone|iPad/.test(navigator.platform)) {
  document.body.classList.add('is-macos');
}
const winMinimize = document.querySelector('#win-minimize');
const winMaximize = document.querySelector('#win-maximize');
const winClose = document.querySelector('#win-close');
if (winMinimize) {
  winMinimize.addEventListener('click', () => { void currentWindow()?.minimize(); });
}
if (winMaximize) {
  winMaximize.addEventListener('click', () => { void currentWindow()?.toggleMaximize(); });
}
if (winClose) {
  winClose.addEventListener('click', () => { void currentWindow()?.close(); });
}
