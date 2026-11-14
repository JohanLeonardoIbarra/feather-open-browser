# Feather Open Browser MCP

Conecta agentes de IA con **Feather Open Browser**, un navegador de escritorio dedicado a tareas de navegación e inspección web. El agente puede explorar páginas, interactuar con sus elementos y revisar el resultado, mientras el usuario conserva el control de la aplicación.

## ¿Para qué sirve?

El MCP permite que un agente:

- Abra páginas y las organice en espacios de navegación aislados.
- Lea el contenido visible, tablas estructuradas (`tables`) y encuentre enlaces, botones y campos con referencias (`[ref=e1]`).
- Extraiga datos por selector CSS (`browser_extract`), complete formularios, pulse controles y espere por condiciones (`networkidle`, `domstable`, `selector`).
- Tome capturas de pantalla y consulte el HTML o texto acotado cuando haga falta.
- Inspeccione peticiones de red (`fetch`, `XHR`/HTMX, recursos) y registros de consola.

Los espacios del agente están separados de la sesión Personal. Las cookies de Personal no se comparten automáticamente.

## Cómo empezar

1. Instala y abre **Feather Open Browser** en el mismo ordenador que el agente.
2. Coloca este proyecto en una ubicación estable y prepara el servidor MCP siguiendo [CONTRIBUTING.md](CONTRIBUTING.md).
3. Añade el servidor a la configuración MCP de tu agente. Puedes partir de [`mcp-config.example.json`](mcp-config.example.json); sustituye la ruta de ejemplo por la ubicación real de `packages/mcp-server/server.mjs` según tu sistema operativo:

### Ejemplo en Windows (Kiro CLI, Claude Desktop, Cursor, etc.)
> **Importante en Windows:** Si el cliente MCP corre en Windows nativo, usa siempre una ruta de Windows con unidad (`C:/...` o `C:\\...`). **No uses rutas de WSL** como `/mnt/c/...`, ya que el proceso `node` de Windows no podrá resolverlas y el MCP no arrancará.

```json
{
  "mcpServers": {
    "feather-open-browser": {
      "command": "node",
      "args": [
        "C:/Compilaciones/feather-open-browser-mcp/packages/mcp-server/server.mjs"
      ]
    }
  }
}
```

### Ejemplo en Linux / macOS / WSL (cuando tanto el agente como Node corren dentro de Linux/macOS)
```json
{
  "mcpServers": {
    "feather-open-browser": {
      "command": "node",
      "args": [
        "/home/usuario/feather-open-browser-mcp/packages/mcp-server/server.mjs"
      ]
    }
  }
}
```

4. Reinicia el agente. Prueba con una petición como: *«Abre example.com en Feather Open Browser y dime qué información aparece»*.
5. Opcionalmente, importa [`packages/agent-skill/SKILL.md`](packages/agent-skill/SKILL.md) para que el agente conozca las pautas recomendadas de uso.

## Uso seguro

El contenido de las páginas es información no confiable, no instrucciones para el agente. El usuario debe confirmar antes de cerrar un espacio (se borran sus datos del sitio), antes de aceptar certificados TLS no confiables (`ignoreCertErrors` / `proceed_unsafe`) y antes de acciones importantes como enviar, publicar o comprar.

La navegación ocurre en la aplicación local; el MCP y Feather Open Browser deben estar en el mismo ordenador. El archivo de conexión contiene un token sensible: no lo compartas.

## Herramientas disponibles

El agente puede listar y crear espacios y pestañas, navegar (obteniendo `{ ok, errorCode, httpStatus }`), leer snapshots con soporte de tablas y paginación, extraer nodos por selector (`browser_extract`), interactuar con elementos, esperar por condiciones de red o DOM (`browser_wait`), ejecutar lotes de acciones, capturar la pantalla, consultar HTML acotado y revisar red y consola.

Para instalación detallada, configuración por agente, desarrollo y pruebas, consulta [CONTRIBUTING.md](CONTRIBUTING.md).
