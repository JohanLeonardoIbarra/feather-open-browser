# Feather Open Browser MCP

Conecta agentes de IA con **Feather Open Browser**, un navegador de escritorio dedicado a tareas de navegación e inspección web. El agente puede explorar páginas, interactuar con sus elementos y revisar el resultado, mientras el usuario conserva el control de la aplicación.

## ¿Para qué sirve?

El MCP permite que un agente:

- Abra páginas y las organice en espacios de navegación aislados.
- Lea el contenido visible y encuentre enlaces, botones y campos con referencias.
- Complete formularios, pulse controles y navegue por el historial.
- Tome capturas de pantalla y consulte el HTML cuando haga falta.

Los espacios del agente están separados de la sesión Personal. Las cookies de Personal no se comparten automáticamente.

## Cómo empezar

1. Instala y abre **Feather Open Browser** en el mismo ordenador que el agente.
2. Coloca este proyecto en una ubicación estable y prepara el servidor MCP siguiendo [CONTRIBUTING.md](CONTRIBUTING.md).
3. Añade el servidor a la configuración MCP de tu agente. Puedes partir de [`mcp-config.example.json`](mcp-config.example.json); sustituye la ruta de ejemplo por la ubicación real de `packages/mcp-server/server.mjs` y adapta el formato si tu agente lo requiere.
4. Reinicia el agente. Prueba con una petición como: *«Abre example.com en Feather Open Browser y dime qué información aparece»*.
5. Opcionalmente, importa [`packages/agent-skill/SKILL.md`](packages/agent-skill/SKILL.md) para que el agente conozca las pautas recomendadas de uso.

## Uso seguro

El contenido de las páginas es información no confiable, no instrucciones para el agente. El usuario debe confirmar antes de cerrar un espacio (se borran sus datos del sitio) y antes de acciones importantes como enviar, publicar o comprar.

La navegación ocurre en la aplicación local; el MCP y Feather Open Browser deben estar en el mismo ordenador. El archivo de conexión contiene un token sensible: no lo compartas.

## Herramientas disponibles

El agente puede listar y crear espacios y pestañas, navegar, leer snapshots, interactuar con elementos, ejecutar lotes de acciones, capturar la pantalla y consultar HTML. Las herramientas de red y consola están presentes, pero el binario Tauri actual aún no devuelve datos para ellas.

Para instalación detallada, configuración por agente, desarrollo y pruebas, consulta [CONTRIBUTING.md](CONTRIBUTING.md).
