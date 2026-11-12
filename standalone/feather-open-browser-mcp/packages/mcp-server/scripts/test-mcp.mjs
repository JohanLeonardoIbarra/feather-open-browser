import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const packageDir = fileURLToPath(new URL('..', import.meta.url));
const serverEntry = fileURLToPath(new URL('../server.mjs', import.meta.url));
const client = new Client({ name: 'feather-open-browser-mcp-test', version: '1.0.0' });
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [serverEntry],
  cwd: packageDir,
  stderr: 'pipe'
});

try {
  await client.connect(transport);
  const { tools } = await client.listTools();
  const names = new Set(tools.map(({ name }) => name));

  for (const expected of [
    'browser_spaces',
    'browser_create_space',
    'browser_snapshot',
    'browser_run_actions',
    'browser_screenshot',
    'browser_get_html'
  ]) {
    assert(names.has(expected), `Missing MCP tool: ${expected}`);
  }

  console.log(`MCP stdio handshake OK (${tools.length} tools).`);
} finally {
  await client.close();
}
