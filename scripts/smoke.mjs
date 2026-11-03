import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { OpenBrowserClient } from '../src/agent-api/client.js';

const projectRoot = join(import.meta.dirname, '..');
const userDataPath = await mkdtemp(join(tmpdir(), 'open-browser-smoke-'));
const configPath = join(userDataPath, 'agent-api.json');
const electronPath = join(projectRoot, 'node_modules', '.bin', 'electron');
const screenshotViewport = process.env.OPEN_BROWSER_VIEWPORT || '1260x677';
const screenshotSize = screenshotViewport.split('x').map(Number);
if (screenshotSize.length !== 2 || screenshotSize.some((size) => !Number.isInteger(size) || size < 320)) {
  throw new Error(`OPEN_BROWSER_VIEWPORT inválido: ${screenshotViewport}`);
}
const fixtureServer = createServer((_request, response) => {
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  response.end(`<!doctype html><html><body>
    <h1>Smoke fixture</h1>
    <form id="search"><label>Query <input aria-label="Query"></label>
      <button id="submit" type="submit">Smoke action</button>
    </form>
    <button id="result" type="button" disabled>Waiting</button>
    <script>document.querySelector('#search').addEventListener('submit', (event) => {
      event.preventDefault();
      document.querySelector('#result').textContent = 'submitted: ' + document.querySelector('input').value;
    });</script>
  </body></html>`);
});
await new Promise((resolve) => fixtureServer.listen(0, '127.0.0.1', resolve));
const fixtureUrl = `http://127.0.0.1:${fixtureServer.address().port}/`;
const child = spawn(electronPath, [
  '--ozone-platform=headless',
  ...(process.env.OPEN_BROWSER_SCREENSHOT_PATH ? [`--ozone-override-screen-size=${screenshotSize.join(',')}`] : []),
  '.',
], {
  cwd: projectRoot,
  env: {
    ...process.env,
    OPEN_BROWSER_USER_DATA: userDataPath,
    ...(process.env.OPEN_BROWSER_SCREENSHOT_PATH ? { OPEN_BROWSER_WINDOW_SIZE: screenshotViewport } : {}),
    ...(process.env.OPEN_BROWSER_SCREENSHOT_PATH ? { OPEN_BROWSER_DEMO_AGENT_SCREEN: '1' } : {}),
    ...(process.env.OPEN_BROWSER_SCREENSHOT_PATH
      ? { OPEN_BROWSER_SCREENSHOT: process.env.OPEN_BROWSER_SCREENSHOT_PATH }
      : {})
  },
  detached: true,
  stdio: ['ignore', 'pipe', 'pipe']
});
let mcpClient;

let logs = '';
let childExited = false;
child.once('exit', () => { childExited = true; });
child.stdout.on('data', (chunk) => { logs += chunk.toString(); });
child.stderr.on('data', (chunk) => { logs += chunk.toString(); });

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let config;
try {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (childExited) throw new Error(`Electron terminó con código ${child.exitCode}.\n${logs}`);
    try {
      config = JSON.parse(await readFile(configPath, 'utf8'));
      break;
    } catch {
      await delay(250);
    }
  }
  if (!config) throw new Error(`La aplicación no inició la API a tiempo.\n${logs}`);

  const browser = new OpenBrowserClient(config);
  console.log('Electron/API iniciados; comprobando estado de espera inicial…');
  const initialSpaces = await browser.spaces();
  if (initialSpaces.length !== 0) {
    throw new Error(`Se esperaban 0 spaces iniciales en standby, pero se encontraron ${initialSpaces.length}.`);
  }
  console.log('API responde en standby; creando Space bajo demanda para el agente y cargando fixture local…');
  const created = await browser.createSpace({ name: 'Smoke test', url: fixtureUrl });
  if (!created.space?.id || !created.tab?.id) throw new Error('No se pudo crear un Space y una pestaña.');
  console.log('Página cargada; solicitando snapshot y comprobando refs…');
  const snapshot = await browser.snapshot(created.space.id, created.tab.id);
  if (!Array.isArray(snapshot.elements) || !snapshot.url) throw new Error('El snapshot inicial no tiene la forma esperada.');
  const input = snapshot.elements.find((element) => element.includes('textbox') && element.includes('Query'));
  const button = snapshot.elements.find((element) => element.includes('button') && element.includes('Smoke action'));
  const inputRef = input?.match(/\[ref=(e\d+)\]/)?.[1];
  const buttonRef = button?.match(/\[ref=(e\d+)\]/)?.[1];
  if (!inputRef || !buttonRef) throw new Error(`No se encontraron los controles esperados: ${JSON.stringify(snapshot.elements)}`);
  const result = await browser.actions(created.space.id, created.tab.id, [
    { type: 'fill', ref: inputRef, value: 'headless works' },
    { type: 'click', ref: buttonRef }
  ]);
  if (!result.snapshot.elements.some((line) => line.includes('submitted: headless works'))) {
    throw new Error(`Las acciones no actualizaron el fixture: ${JSON.stringify(result.snapshot.elements)}`);
  }

  const mcpTransport = new StdioClientTransport({
    command: process.execPath,
    args: [join(projectRoot, 'scripts', 'mcp-server.mjs')],
    cwd: projectRoot,
    env: { ...process.env, OPEN_BROWSER_CONFIG: configPath },
    stderr: 'pipe'
  });
  mcpClient = new Client({ name: 'open-browser-smoke', version: '1.0.0' });
  await mcpClient.connect(mcpTransport);
  const toolNames = (await mcpClient.listTools()).tools.map((tool) => tool.name);
  for (const expected of [
    'browser_spaces', 'browser_snapshot', 'browser_click', 'browser_fill', 'browser_press',
    'browser_run_actions', 'browser_screenshot', 'browser_get_html', 'browser_network_requests', 'browser_console_logs'
  ]) {
    if (!toolNames.includes(expected)) throw new Error(`No se registró la herramienta MCP ${expected}.`);
  }

  const unwrap = (toolResult) => toolResult.structuredContent ?? JSON.parse(toolResult.content[0].text);
  const spacesResult = unwrap(await mcpClient.callTool({ name: 'browser_spaces', arguments: {} }));
  if (!spacesResult.items.some((space) => space.id === created.space.id)) throw new Error('MCP no encontró el Space de prueba.');

  const mcpSnapshot = unwrap(await mcpClient.callTool({
    name: 'browser_snapshot',
    arguments: { spaceId: created.space.id, tabId: created.tab.id }
  }));
  const mcpInput = mcpSnapshot.elements.find((element) => element.includes('textbox') && element.includes('Query'));
  const mcpInputRef = mcpInput?.match(/\[ref=(e\d+)\]/)?.[1];
  const fillResult = unwrap(await mcpClient.callTool({
    name: 'browser_fill',
    arguments: { spaceId: created.space.id, tabId: created.tab.id, ref: mcpInputRef, value: 'via MCP' }
  }));
  const pressResult = unwrap(await mcpClient.callTool({
    name: 'browser_press',
    arguments: { spaceId: created.space.id, tabId: created.tab.id, key: 'Enter' }
  }));
  if (!pressResult.snapshot.elements.some((element) => element.includes('submitted: via MCP'))) {
    throw new Error('La herramienta MCP browser_press no envió Enter al formulario.');
  }
  const secondInput = pressResult.snapshot.elements.find((element) => element.includes('textbox') && element.includes('Query'));
  const secondInputRef = secondInput?.match(/\[ref=(e\d+)\]/)?.[1];
  const secondFill = unwrap(await mcpClient.callTool({
    name: 'browser_fill',
    arguments: { spaceId: created.space.id, tabId: created.tab.id, ref: secondInputRef, value: 'via MCP click' }
  }));
  const mcpButton = secondFill.snapshot.elements.find((element) => element.includes('button') && element.includes('Smoke action'));
  const mcpButtonRef = mcpButton?.match(/\[ref=(e\d+)\]/)?.[1];
  const clickResult = unwrap(await mcpClient.callTool({
    name: 'browser_click',
    arguments: { spaceId: created.space.id, tabId: created.tab.id, ref: mcpButtonRef }
  }));
  if (!clickResult.snapshot.elements.some((element) => element.includes('submitted: via MCP click'))) {
    throw new Error('Las herramientas MCP no pudieron completar la interacción de la página fixture.');
  }

  console.log('Comprobando herramientas de verificación (screenshot, html, network, console)…');
  const shotResult = unwrap(await mcpClient.callTool({
    name: 'browser_screenshot',
    arguments: { spaceId: created.space.id, tabId: created.tab.id }
  }));
  if (!shotResult.dataUrl?.startsWith('data:image/png;base64,')) {
    throw new Error('browser_screenshot no devolvió un dataUrl PNG válido.');
  }

  const htmlResult = unwrap(await mcpClient.callTool({
    name: 'browser_get_html',
    arguments: { spaceId: created.space.id, tabId: created.tab.id }
  }));
  if (!htmlResult.html?.includes('Smoke fixture')) {
    throw new Error('browser_get_html no devolvió el HTML esperado.');
  }

  const netResult = unwrap(await mcpClient.callTool({
    name: 'browser_network_requests',
    arguments: { spaceId: created.space.id, tabId: created.tab.id }
  }));
  if (!Array.isArray(netResult.requests)) {
    throw new Error('browser_network_requests no devolvió una lista de peticiones.');
  }

  const consoleResult = unwrap(await mcpClient.callTool({
    name: 'browser_console_logs',
    arguments: { spaceId: created.space.id, tabId: created.tab.id }
  }));
  if (!Array.isArray(consoleResult.logs)) {
    throw new Error('browser_console_logs no devolvió una lista de registros.');
  }

  await mcpClient.callTool({ name: 'browser_close_space', arguments: { spaceId: created.space.id } });
  const remaining = await browser.spaces();
  if (remaining.length !== 0) {
    throw new Error(`El agente cerró el Space pero quedaron ${remaining.length} spaces.`);
  }
  console.log('Smoke test OK: Navegador exclusivo de agentes, standby inicial, Space bajo demanda, acciones, verificación visual/red/consola y cierre por el agente.');
} catch (error) {
  console.error(`Smoke test failed. Electron output:\n${logs}`);
  throw error;
} finally {
  if (mcpClient) {
    try { await mcpClient.close(); } catch {}
  }
  if (!childExited) {
    try { process.kill(-child.pid, 'SIGTERM'); } catch {}
    await Promise.race([
      new Promise((resolve) => child.once('exit', resolve)),
      delay(5_000)
    ]);
    if (!childExited) {
      try { process.kill(-child.pid, 'SIGKILL'); } catch {}
      await Promise.race([new Promise((resolve) => child.once('exit', resolve)), delay(1_000)]);
    }
  }
  await new Promise((resolve) => fixtureServer.close(resolve));
  await rm(userDataPath, { recursive: true, force: true });
}
