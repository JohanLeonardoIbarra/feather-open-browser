---
name: open-browser
description: Control the local Open Browser app using its authenticated localhost API. Dedicated agent-exclusive browser environment for inspecting, developing, and debugging web applications.
---

# Open Browser

Open Browser is a browser environment dedicated exclusively to AI agents. It starts in standby with 0 spaces and spawns ephemeral Spaces strictly on-demand when an agent requests one. The agent is responsible for closing its space when its task or feedback loop is complete.

## Connect

Prefer the MCP tools:
- **Lifecycle**: `browser_spaces`, `browser_create_space`, `browser_close_space`, `browser_tabs`, `browser_open_tab`, `browser_close_tab`.
- **Navigation & Actions**: `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_fill`, `browser_press`, `browser_wait`, `browser_back`, `browser_forward`, `browser_reload`, `browser_run_actions`.
- **Verification & Debugging**: `browser_screenshot`, `browser_get_html`, `browser_network_requests`, `browser_console_logs`.

Project-local MCP configuration is available for OpenCode (`opencode.json`) and Antigravity (`.agents/plugins/open-browser/mcp_config.json`); other MCP clients can run `node scripts/mcp-server.mjs` from the checkout root. Keep the Open Browser app running.

## Verification Policy

All verification and assertions in Open Browser must be done exclusively through browser observables:
- **HTML & DOM Snapshots**: Use `browser_snapshot` for interactive elements and text, or `browser_get_html` for full document or ref outerHTML.
- **Visual Screenshots**: Use `browser_screenshot` to inspect visual layouts, detect design regressions, or verify elements that are obscured or difficult to access in DOM.
- **Terminal & Console Logs**: Use `browser_console_logs` to verify client-side errors, warnings, and messages.
- **Network Requests**: Use `browser_network_requests` to verify API calls, status codes, and network payloads.
- **Strict Prohibition**: Never validate behavior using terminal commands like `docker` or by connecting directly to databases. Always observe what the user sees in the browser.

## Typical Flow: Inspection & Feedback Loop

1. **Spawn Space**: Create a space on demand for the application under test:
   `browser_create_space({ name: 'Dev Server', url: 'http://localhost:3000' })`
2. **Scan & Plan**: Request a snapshot (`browser_snapshot`) to inspect available interactive refs (`e1`, `e2`, etc.). Formulate your action plan.
3. **Execute Visually**: Run actions via `browser_run_actions` (or individual tools). The virtual ghost cursor, click ripple, and element focus render live on screen.
4. **Observe & Verify**: Use `browser_screenshot`, `browser_console_logs`, `browser_network_requests`, and `browser_get_html` to detect layout bugs, failing requests, or script errors.
5. **Self-Correct**: If errors or visual regressions are found, adjust the codebase and re-verify.
6. **Clean Up**: When the task is complete, close the space:
   `browser_close_space({ spaceId })`. The browser returns to standby.

```js
import { OpenBrowserClient } from './src/agent-api/client.js';
const browser = await OpenBrowserClient.connect();

// 1. Create on-demand space
const { space, tab } = await browser.createSpace({
  name: 'App Verification',
  url: 'http://localhost:3000'
});

// 2. Observe initial state
const page = await browser.snapshot(space.id, tab.id);

// 3. Interact visually
await browser.actions(space.id, tab.id, [
  { type: 'fill', ref: 'e1', value: 'admin' },
  { type: 'click', ref: 'e2' }
]);

// 4. Verify observables
const shot = await browser.screenshot(space.id, tab.id);
const logs = await browser.consoleLogs(space.id, tab.id);
const requests = await browser.network(space.id, tab.id);

// 5. Close space when finished
await browser.closeSpace(space.id);
```

Supported batched actions: `navigate` (`url`), `click` (`ref`), `fill` (`ref`, `value`), `press` (`key`), `wait` (`ms`), `back`, `forward`, and `reload`. Keep batches at 32 actions or fewer. Navigation targets must be valid HTTP/HTTPS URLs (e.g. `http://localhost:3000`, `http://127.0.0.1:8080`, `https://...`); there is no search engine.
