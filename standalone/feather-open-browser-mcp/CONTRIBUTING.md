# Contribuir a Feather Open Browser MCP

Este repositorio mantiene la integración MCP y la Skill de agente para Feather Open Browser. **No contiene ni compila la aplicación de escritorio**; para probar las herramientas de navegación se necesita tener Feather Open Browser instalada y abierta localmente.

## Requisitos de desarrollo

- Node.js 22 o posterior.
- npm.
- Git.
- Para pruebas manuales de las herramientas: la aplicación Feather Open Browser en el mismo equipo.

## Preparar el monorepo

Desde la raíz del repositorio:

```bash
npm ci
npm run check
npm test
```

Los workspaces son:

- `packages/mcp-server/`: servidor MCP por stdio y cliente HTTP de la API local.
- `packages/agent-skill/`: Skill con pautas de interacción segura y eficiente.

`npm test` inicia el servidor MCP como subproceso y verifica el handshake y un conjunto de herramientas. No requiere abrir el navegador. Para iniciar el servidor manualmente, usa `npm start`; el protocolo MCP usa stdin/stdout, por lo que no se deben imprimir mensajes de diagnóstico en stdout.

## Configuración de agentes

`mcp-config.example.json` muestra el formato común `mcpServers`. Cada cliente MCP puede tener su propia estructura; la configuración debe ejecutar Node con `packages/mcp-server/server.mjs` como argumento. Usa una ruta absoluta en la máquina donde corre el agente. En OpenCode también puede configurarse con `type: "local"`, `command: ["node", "/ruta/absoluta/packages/mcp-server/server.mjs"]` y `cwd` apuntando a la raíz del monorepo.

El cliente MCP descubre automáticamente `agent-api.json` en la carpeta de configuración del usuario:

- Windows: `%APPDATA%\feather-open-browser\agent-api.json`
- macOS: `~/Library/Application Support/feather-open-browser/agent-api.json`
- Linux: `$XDG_CONFIG_HOME/feather-open-browser/agent-api.json` o `~/.config/feather-open-browser/agent-api.json`

Si se usa otra ubicación, define `FEATHER_OPEN_BROWSER_CONFIG` en el entorno del proceso MCP. El archivo se crea al iniciar la aplicación y contiene el token de control local; no se debe copiar, publicar ni registrar en logs.

## Arquitectura

- `packages/mcp-server/server.mjs` inicia el servidor MCP sobre stdio.
- `packages/mcp-server/mcp-server.js` registra las herramientas, valida sus argumentos y los transforma en llamadas al API local.
- `packages/mcp-server/client.js` localiza la configuración temporal del API y realiza las solicitudes autenticadas.
- `packages/agent-skill/SKILL.md` explica al agente cómo usar las herramientas y cómo tratar contenido web no confiable.

El servidor MCP no inicia ni controla el ciclo de vida de la aplicación de escritorio: solo consume su API local autenticada. No se abre un puerto MCP de red.

## Mantener el paquete

El código del cliente y del servidor es una copia autónoma de la integración del repositorio principal `feather-open-browser`. Al cambiar el protocolo, las rutas o las herramientas en la aplicación, sincroniza `client.js` y `mcp-server.js` y actualiza este paquete y su Skill.

Las herramientas `browser_network_requests` y `browser_console_logs` están registradas, pero el binario Tauri actual responde con listas vacías. No presentes esas herramientas como diagnósticos funcionales hasta que el API del navegador las implemente.

Las pull requests deben incluir pruebas o actualizar las existentes. El workflow de GitHub Actions ejecuta `npm ci`, `npm run check` y `npm test` con Node.js 22.
