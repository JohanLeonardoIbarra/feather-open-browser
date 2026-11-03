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
