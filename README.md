# Open Browser

Navegador local para Linux con pestañas personales y Spaces separados para agentes. Es una implementación independiente; no contiene ni reutiliza el binario de ego-lite.

## Estado del MVP

- Ventana de navegador Electron/Chromium con navegación, pestañas y Spaces.
- El perfil Personal persiste en disco; los Spaces de agentes son efímeros y aislados, y se pueden cerrar borrando sus datos del sitio.
- Creación manual de Space con copia opcional, explícita y puntual de cookies desde Personal. No copia `localStorage`; las cookies particionadas se omiten.
- API HTTP local autenticada para que un agente cree Spaces y pestañas, navegue, obtenga snapshots y ejecute lotes acotados de acciones.
- SDK JavaScript y borrador de Skill para conectar agentes desde este repositorio.
- Servidor MCP por stdio con herramientas de Spaces, pestañas, navegación, snapshot e interacción.
- El snapshot inicial cubre el documento principal; el recorrido de iframes aún está pendiente.

## Desarrollo

Requiere Node.js 22 o posterior y npm.

```bash
npm install
npm start
```

La API escucha solo en `127.0.0.1` y usa un puerto aleatorio. Al iniciar, escribe temporalmente sus datos de conexión en `agent-api.json` dentro del directorio `userData` de Electron, con permisos de usuario. El archivo se elimina al cerrar la app. No compartas ese token: permite controlar páginas y sesiones abiertas.

El proyecto incluye `opencode.json` (para OpenCode) y `.agents/plugins/open-browser/` junto con `.agents/skills.json` (para Antigravity) para conectar al servidor MCP local por stdio de forma local al proyecto. Para otros agentes MCP, ejecuta `node scripts/mcp-server.mjs` desde la raíz del checkout. El navegador debe estar abierto para usar las herramientas.

Herramientas MCP: `browser_spaces`, `browser_create_space`, `browser_close_space`, `browser_tabs`, `browser_open_tab`, `browser_close_tab`, `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_fill`, `browser_press`, `browser_wait`, `browser_back`, `browser_forward`, `browser_reload` y `browser_run_actions`.

Rutas v1 principales:

- `GET /v1/spaces`
- `POST /v1/spaces` con `{ "name": "Investigación", "url": "https://example.com" }`
- `GET /v1/spaces/:spaceId/tabs/:tabId/snapshot`
- `POST /v1/spaces/:spaceId/tabs/:tabId/navigate` con `{ "url": "https://example.com" }`
- `POST /v1/spaces/:spaceId/tabs/:tabId/actions` con `{ "actions": [{ "type": "click", "ref": "e1" }] }`
- `DELETE /v1/spaces/:spaceId` (borra cookies y datos efímeros del Space)

Las rutas, salvo `/health`, requieren `Authorization: Bearer <token>`.

## Empaquetado y Distribución

Open Browser soporta compilación y distribución multiplataforma con `electron-builder`:

### 1. Linux (AppImage)
Genera un binario autocontenido `.AppImage` para `x86_64` con iconos integrados y soporte de escritorio:
```bash
npm run dist:linux
# O simplemente:
npm run dist
```
Salida en: `dist/Open Browser-0.1.0-x86_64.AppImage`

### 2. Windows (Portable .exe)
Genera un ejecutable portable independiente (sin necesidad de instalador) para Windows `x64`:
```bash
npm run dist:win
```
Salida en: `dist/Open Browser-0.1.0-portable-x64.exe`

### 3. macOS (DMG y ZIP)
Genera el paquete `.dmg` con ventana de instalación a `/Applications` y el `.zip` con `Open Browser.app` para Intel (`x64`) y Apple Silicon (`arm64`):
```bash
npm run dist:mac
```
> **Nota para macOS:** La creación de imágenes de disco `.dmg` con estilo de Finder requiere macOS (o el flujo de CI en GitHub Actions con `macos-latest`), ya que utiliza herramientas nativas de Apple (`sips`, `dmgbuild`, `hdiutil`). En Linux, se puede empaquetar el bundle `Open Browser.app` y su `.zip` mediante `npx electron-builder --mac zip`.

### 4. CI/CD Automatizado
El repositorio incluye un flujo de GitHub Actions en `.github/workflows/build.yml` que compila en paralelo las tres plataformas (Linux AppImage, Windows Portable y macOS DMG/ZIP) al hacer push o crear un tag de versión.
