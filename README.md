<div align="center">
  <img src="build/icon.svg" width="128" height="128" alt="Feather Open Browser logo">
  <h1>Feather Open Browser</h1>
  <p><strong>Un navegador local para que los agentes de IA puedan explorar, probar e inspeccionar la web.</strong></p>
  <p>
    <a href="https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/latest">Descargar</a>
    ·
    <a href="#cómo-empezar">Conectar un agente</a>
    ·
    <a href="CONTRIBUTING.md">Contribuir</a>
  </p>
  <p>
    <img src="https://img.shields.io/github/v/release/JohanLeonardoIbarra/feather-open-browser?style=for-the-badge&label=latest%20release" alt="Última versión">
  </p>
</div>

---

Feather Open Browser ofrece a los agentes un navegador dedicado, mientras mantiene separado el perfil Personal. Sirve para investigar sitios, probar flujos web y revisar visualmente páginas durante tareas de desarrollo o QA.

## Descargas

Última versión publicada: **v0.0.4**. Los enlaces descargan directamente los binarios de esa versión; [abre la página de Releases](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/latest) para comprobar la versión más reciente.

| Sistema | Descargas |
| --- | --- |
| **Windows x64** | [Instalador `.exe`](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/Feather.Open.Browser_0.0.4_x64-setup.exe) · [MSI](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/Feather.Open.Browser_0.0.4_x64_en-US.msi) · [Portable `.exe`](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/feather-open-browser-portable.exe) · [ZIP portable](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/feather-open-browser-windows-x64-portable.zip) |
| **macOS Universal** | [DMG](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/Feather.Open.Browser_0.0.4_universal.dmg) |
| **Linux x64** | [AppImage](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/Feather.Open.Browser_0.0.4_amd64.AppImage) · [DEB](https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/download/v0.0.4/Feather.Open.Browser_0.0.4_amd64.deb) |

<p align="center">
  <a href="https://github.com/JohanLeonardoIbarra/feather-open-browser/releases/latest"><strong>Ver todas las versiones y descargas →</strong></a>
</p>

## Qué puedes hacer

<table>
  <tr>
    <td width="50%"><strong>🧭 Navegar</strong><br>Abre páginas y organiza el trabajo en espacios y pestañas.</td>
    <td width="50%"><strong>🧩 Interactuar</strong><br>Lee controles visibles, completa campos y pulsa elementos de una página.</td>
  </tr>
  <tr>
    <td><strong>🔍 Inspeccionar</strong><br>Obtén snapshots, HTML y capturas para entender o revisar una página.</td>
    <td><strong>🛡️ Mantener separado</strong><br>Los espacios del agente están aislados del perfil Personal; compartir cookies requiere una acción explícita.</td>
  </tr>
</table>

## Cómo empezar

1. Descarga e instala Feather Open Browser para tu sistema operativo.
2. Abre Feather Open Browser y pulsa la **tuerca ⚙️ de la barra superior**: ahí tienes la configuración MCP (JSON con la URL y el token de la app en marcha) y el texto de la Skill, listos para copiar.
3. Pega el JSON en la configuración de servidores MCP de tu agente y, si quieres, añade la Skill como `SKILL.md`.
4. Prueba con una petición como: *«Abre example.com en Feather Open Browser y resume lo que aparece»*.

El MCP está integrado en la propia aplicación: se sirve por HTTP local en `http://127.0.0.1:<puerto>/mcp` y se autentica con el token Bearer que muestra la tuerca. El token cambia en cada arranque de la app, por lo que hay que regenerar la configuración si lo copiaste antes.

## Privacidad y uso seguro

- La conexión entre el agente y la aplicación es local.
- El contenido de los sitios web es información no confiable, no instrucciones para el agente.
- Cerrar un espacio borra sus cookies y datos del sitio; confirma antes de hacerlo.
- Confirma también antes de enviar, publicar, comprar o realizar otra acción importante.

## Estado

La navegación (con reporte de estado y errores de red/TLS), interacción con páginas (compatible con Trusted Types y HTMX), snapshots con tablas estructuradas, extracción por selector CSS, HTML acotado, capturas de pantalla y diagnósticos de red/consola están disponibles.

<p align="center"><sub>¿Quieres desarrollar o modificar el proyecto? Consulta <a href="CONTRIBUTING.md">Contributing</a>.</sub></p>
