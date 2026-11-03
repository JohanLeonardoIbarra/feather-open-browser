export function normalizeNavigationTarget(input, { allowFile = false } = {}) {
  const value = String(input ?? '').trim();
  if (!value) throw new Error('La dirección está vacía.');
  if (value === 'about:blank') return value;

  let candidate = value;
  if (!/^[a-zA-Z][a-zA-Z\d+.-]*:\/\//.test(candidate)) {
    // If it starts with localhost or an IP, default to http://
    if (/^(localhost|\d{1,3}(\.\d{1,3}){3})(:\d+)?(\/.*)?$/i.test(candidate)) {
      candidate = `http://${candidate}`;
    } else if (!/^[a-zA-Z][a-zA-Z\d+.-]*:/.test(candidate) || /^[a-zA-Z0-9.-]+:\d+/.test(candidate)) {
      candidate = `https://${candidate}`;
    }
  }

  let parsed;
  try {
    parsed = new URL(candidate);
  } catch {
    throw new Error('La dirección no es una URL válida.');
  }

  const allowed = allowFile ? ['http:', 'https:', 'file:'] : ['http:', 'https:'];
  if (!allowed.includes(parsed.protocol)) {
    throw new Error(`Protocolo no permitido: ${parsed.protocol}`);
  }
  return parsed.href;
}
