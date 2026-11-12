# Contribuir a Feather Open Browser

Esta guía contiene los detalles técnicos del proyecto principal: la aplicación de escritorio Feather Open Browser y su integración con agentes. El paquete MCP independiente se mantiene también en [feather-open-browser-mcp](https://github.com/JohanLeonardoIbarra/feather-open-browser-mcp).

## Estructura

- `src-tauri/`: aplicación nativa Tauri v2, API local autenticada y automatización del webview.
- `src/ui/`: interfaz de usuario del navegador.
- `src/agent-api/`: cliente y servidor MCP de Node.js usados desde este repositorio.
- `skills/open-browser/SKILL.md`: Skill de agente incluida en el proyecto.
- `standalone/feather-open-browser-mcp/`: copia de trabajo del paquete MCP independiente; sincronizarla al cambiar el cliente, las herramientas o la Skill.

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

Para una ruta personalizada se puede definir `FEATHER_OPEN_BROWSER_CONFIG`. El MCP se ejecuta por stdio y consume la API del navegador; no abre un puerto de red propio. En el cliente de agente, configura la ruta local al script MCP correspondiente.

La configuración de Antigravity en `.agents/plugins/open-browser/mcp_config.json` puede contener rutas locales específicas de la máquina; actualízalas al cambiar de equipo.

### Limitaciones conocidas

- El binario Tauri actual responde con listas vacías en las rutas de red y consola; las herramientas `browser_network_requests` y `browser_console_logs` están registradas, pero aún no proporcionan diagnósticos.
- El paquete MCP independiente contiene copias autónomas del cliente y servidor de este repositorio. Si cambia el protocolo o el esquema de herramientas, actualiza y prueba ambos repositorios.

## Contribuciones

Antes de enviar una pull request, ejecuta `npm test` y `npm run check`. Para cambios en el API o MCP, añade o actualiza pruebas y mantén sincronizadas la Skill y la distribución independiente.
