import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { buildVisualActionScript, ALLOWED_KEYS } from '../src/browser/visual-agent.js';

test('buildVisualActionScript exports valid action keys', () => {
  assert.ok(ALLOWED_KEYS.has('Enter'));
  assert.ok(ALLOWED_KEYS.has('Tab'));
  assert.ok(ALLOWED_KEYS.has('Space'));
  assert.ok(ALLOWED_KEYS.has('Escape'));
  assert.ok(!ALLOWED_KEYS.has('F12'));
});

test('buildVisualActionScript produces valid JavaScript syntax for click, fill, and press without innerHTML (Trusted Types safe)', async () => {
  const clickScript = buildVisualActionScript({ type: 'click', ref: 'e1' }, { fast: false });
  assert.match(clickScript, /__open_browser_cursor/);
  assert.match(clickScript, /ob-cursor-inner/);
  assert.match(clickScript, /ob-cursor-trail/);
  assert.match(clickScript, /ob-click-ripple/);
  assert.doesNotMatch(clickScript, /\.innerHTML\s*=/);
  assert.doesNotThrow(() => new Function(clickScript));

  const fillScript = buildVisualActionScript({ type: 'fill', ref: 'e2', value: 'pikachu' }, { fast: true });
  assert.match(fillScript, /fast = true/);
  assert.doesNotMatch(fillScript, /\.innerHTML\s*=/);
  assert.doesNotThrow(() => new Function(fillScript));

  const pressScript = buildVisualActionScript({ type: 'press', key: 'Enter' }, { fast: false });
  assert.match(pressScript, /KeyboardEvent/);
  assert.doesNotMatch(pressScript, /\.innerHTML\s*=/);
  assert.doesNotThrow(() => new Function(pressScript));

  const tauriVisualAgentPath = fileURLToPath(new URL('../src-tauri/src/visual_agent.js', import.meta.url));
  const tauriTemplate = await readFile(tauriVisualAgentPath, 'utf8');
  assert.doesNotMatch(tauriTemplate, /\.innerHTML\s*=/);
  const hydratedTauriScript = tauriTemplate
    .replace('__ACTION_JSON__', JSON.stringify({ type: 'click', ref: 'e3' }))
    .replace('__FAST_BOOL__', 'false');
  assert.doesNotThrow(() => new Function(hydratedTauriScript));
});
