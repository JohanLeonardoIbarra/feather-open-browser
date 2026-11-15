# Contribuir a Feather Open Browser

Esta guía contiene los detalles técnicos del proyecto principal: la aplicación de escritorio Feather Open Browser y su integración con agentes. El MCP está integrado en la propia aplicación; ya no existe un paquete independiente.

## Estructura

- `src-tauri/`: aplicación nativa Tauri v2, API local autenticada, servidor MCP HTTP (`src/mcp.rs`) y automatización del webview.
- `src/ui/`: interfaz de usuario del navegador, incluido el panel de configuración del agente (tuerca ⚙️).
- `src/agent-api/`: cliente de Node.js (heredado, usado por las pruebas y la versión Electron).
- `skills/open-browser/SKILL.md`: Skill de agente incluida en el proyecto (se expone en el panel de configuración).

El binario de escritorio actual se construye con Tauri. Quedan archivos y dependencias heredados de Electron; no asumir que todos los scripts antiguos representan la implementación activa.

## Requisitos y desarrollo

- Node.js 22 o posterior y npm.
- Rust estable y las dependencias de sistema requeridas por Tauri para el sistema operativo.
- En Linux, bibliotecas GTK y WebKitGTK de desarrollo para compilar la aplicación.

Instala las dependencias y ejecuta la aplicación en modo desarrollo:

```bash
npm ci
npm start
```

Comprobaciones automatizadas:

```bash
npm test
npm run check
```

`npm run check` comprueba sintaxis JavaScript y ejecuta `cargo check`. `npm test` ejecuta las pruebas de Node.js.

`npm run smoke` es un test heredado que lanza Electron; todavía no valida la aplicación Tauri actual y no debe considerarse una prueba de integración del binario publicado.

## Compilación y publicación

Scripts disponibles en `package.json`:

- `npm run tauri:build`: compilar para la plataforma actual.
- `npm run build:appimage`: generar bundles Linux AppImage y deb.
- `npm run build:dmg`: generar un bundle macOS DMG.
- `npm run build:win`: compilar el bundle Windows.

El workflow `.github/workflows/release.yml` crea artefactos multiplataforma al publicar tags `v*`. Instala las dependencias de sistema indicadas por Tauri antes de compilar en cada plataforma.

## API local y MCP

Al arrancar, la aplicación abre una API HTTP ligada únicamente a `127.0.0.1`, en un puerto aleatorio, y crea un token de acceso temporal en `agent-api.json`. El archivo se elimina al cerrar la app. No lo compartas ni lo incluyas en logs: permite controlar páginas y sesiones locales.

El cliente busca el archivo automáticamente en:

- Windows: `%APPDATA%\feather-open-browser\agent-api.json`
- macOS: `~/Library/Application Support/feather-open-browser/agent-api.json`
- Linux: `$XDG_CONFIG_HOME/feather-open-browser/agent-api.json` o `~/.config/feather-open-browser/agent-api.json`

Para una ruta personalizada se puede definir `FEATHER_OPEN_BROWSER_CONFIG`.

### Servidor MCP integrado (HTTP)

La aplicación expone el protocolo MCP (JSON-RPC 2.0, Streamable HTTP) en la misma API local:

- Endpoint: `POST http://127.0.0.1:<puerto>/mcp`
- Autenticación: `Authorization: Bearer <token>` (el mismo token de `agent-api.json`)
- Métodos soportados: `initialize`, `ping`, `tools/list`, `tools/call`; las notificaciones responden `202`.
- Herramientas: las mismas 21 (`browser_spaces`, `browser_create_space`, …, `browser_console_logs`), definidas en `src-tauri/src/mcp.rs`.

La tuerca ⚙️ de la barra superior de la app muestra el JSON de configuración listo para pegar en el cliente MCP:

```json
{
  "mcpServers": {
    "feather-open-browser": {
      "type": "http",
      "url": "http://127.0.0.1:<puerto>/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

El puerto y el token cambian en cada arranque de la app: copia siempre la configuración desde la tuerca con la app abierta. El texto de la Skill (`skills/open-browser/SKILL.md`) también se copia desde ese panel y se incluye embebido en el binario (`include_str!`), por lo que debe mantenerse actualizado en ese archivo.

## Contribuciones

Antes de enviar una pull request, ejecuta `npm test` y `npm run check`. Para cambios en el API, el MCP o las herramientas, añade o actualiza pruebas y mantén la Skill (`skills/open-browser/SKILL.md`) sincronizada con el comportamiento real.
