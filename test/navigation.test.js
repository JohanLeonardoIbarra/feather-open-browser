import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeNavigationTarget } from '../src/browser/navigation.js';

test('adds https to standard domain names', () => {
  assert.equal(normalizeNavigationTarget('example.com'), 'https://example.com/');
});

test('adds http to local development hosts and ports', () => {
  assert.equal(normalizeNavigationTarget('localhost:3000'), 'http://localhost:3000/');
  assert.equal(normalizeNavigationTarget('127.0.0.1:8080/app'), 'http://127.0.0.1:8080/app');
});

test('rejects executable and unsupported protocols', () => {
  assert.throws(() => normalizeNavigationTarget('javascript:alert(1)'), /Protocolo no permitido/);
  assert.throws(() => normalizeNavigationTarget('file:///etc/passwd'), /Protocolo no permitido/);
});

test('allows local files only when explicitly requested', () => {
  assert.equal(normalizeNavigationTarget('file:///tmp/page.html', { allowFile: true }), 'file:///tmp/page.html');
});
