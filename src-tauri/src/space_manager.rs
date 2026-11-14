use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewBuilder, WebviewUrl};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::state::{
  BrowserState, ExtractOptions, GetHtmlOptions, NavigationStatus, SnapshotOptions, SpaceState,
  TabState,
};

pub const TOOLBAR_HEIGHT: f64 = 76.0;

pub const INIT_DIAGNOSTICS_SCRIPT: &str = r#"
(function() {
  if (window.__openBrowserDiagnosticsInstalled) return;
  window.__openBrowserDiagnosticsInstalled = true;

  const diag = window.__openBrowserDiag = window.__openBrowserDiag || {
    consoleLogs: [],
    networkRequests: [],
    inFlightRequests: 0,
    htmxInFlight: 0,
    lastNetworkActivityAt: Date.now(),
    mutationCount: 0,
    lastMutationAt: Date.now(),
    nextReqId: 1
  };

  const pushLog = (level, args) => {
    try {
      const msg = Array.from(args).map((a) => {
        if (typeof a === 'string') return a;
        if (a instanceof Error) return a.stack || a.message || String(a);
        try { return JSON.stringify(a); } catch (_) { return String(a); }
      }).join(' ');
      diag.consoleLogs.push({
        level,
        message: msg.slice(0, 2000),
        timestamp: Date.now()
      });
      if (diag.consoleLogs.length > 200) diag.consoleLogs.shift();
    } catch (_) {}
  };

  ['log', 'info', 'warn', 'error'].forEach((fn) => {
    const orig = console[fn];
    if (typeof orig === 'function') {
      console[fn] = function() {
        pushLog(fn === 'warn' ? 'warning' : fn, arguments);
        return orig.apply(this, arguments);
      };
    }
  });

  window.addEventListener('error', (e) => {
    pushLog('error', [e.message || 'Runtime Error', e.filename || '', e.lineno || '']);
  });

  window.addEventListener('unhandledrejection', (e) => {
    pushLog('error', ['UnhandledRejection:', e.reason]);
  });

  window.addEventListener('beforeunload', () => {
    try {
      window.__openBrowserUnloading = Date.now();
    } catch (_) {}
  });

  const recordRequestStart = (method, url, resourceType) => {
    const entry = {
      id: diag.nextReqId++,
      method: String(method || 'GET').toUpperCase(),
      url: String(url || ''),
      resourceType: resourceType || 'fetch',
      status: 'pending',
      ok: null,
      timestamp: Date.now()
    };
    diag.networkRequests.push(entry);
    if (diag.networkRequests.length > 200) diag.networkRequests.shift();
    diag.inFlightRequests = Math.max(0, (diag.inFlightRequests || 0) + 1);
    diag.lastNetworkActivityAt = Date.now();
    return entry;
  };

  const recordRequestEnd = (entry, status, ok, errorMsg) => {
    diag.inFlightRequests = Math.max(0, (diag.inFlightRequests || 1) - 1);
    diag.lastNetworkActivityAt = Date.now();
    if (entry) {
      entry.status = status;
      entry.ok = ok;
      entry.durationMs = Date.now() - entry.timestamp;
      if (errorMsg) entry.error = String(errorMsg).slice(0, 500);
    }
  };

  if (typeof window.fetch === 'function') {
    const origFetch = window.fetch;
    window.fetch = function(input, init) {
      let url = '';
      let method = (init && init.method) || 'GET';
      try {
        if (typeof input === 'string') url = input;
        else if (input && typeof input.url === 'string') {
          url = input.url;
          if (input.method) method = input.method;
        } else url = String(input);
      } catch (_) {}
      const entry = recordRequestStart(method, url, 'fetch');
      return origFetch.apply(this, arguments).then(
        (res) => {
          recordRequestEnd(entry, res.status, res.ok, null);
          return res;
        },
        (err) => {
          recordRequestEnd(entry, 'error', false, err && (err.message || String(err)));
          throw err;
        }
      );
    };
  }

  if (typeof XMLHttpRequest !== 'undefined') {
    const origOpen = XMLHttpRequest.prototype.open;
    const origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url) {
      this.__obMethod = method || 'GET';
      this.__obUrl = url || '';
      return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function() {
      const entry = recordRequestStart(this.__obMethod, this.__obUrl, 'xhr');
      const onDone = () => {
        const status = this.status || 'error';
        const ok = typeof this.status === 'number' && this.status >= 200 && this.status < 400;
        recordRequestEnd(entry, status, ok, ok ? null : (this.statusText || null));
      };
      this.addEventListener('loadend', onDone, { once: true });
      return origSend.apply(this, arguments);
    };
  }

  // HTMX lifecycle listeners for accurate settle detection
  ['htmx:beforeRequest', 'htmx:configRequest'].forEach((evt) => {
    window.addEventListener(evt, () => {
      if (evt === 'htmx:beforeRequest') {
        diag.htmxInFlight = Math.max(0, (diag.htmxInFlight || 0) + 1);
      }
      diag.lastNetworkActivityAt = Date.now();
    }, true);
  });

  ['htmx:afterSettle', 'htmx:responseError', 'htmx:sendError', 'htmx:timeout', 'htmx:swapError'].forEach((evt) => {
    window.addEventListener(evt, () => {
      diag.htmxInFlight = Math.max(0, (diag.htmxInFlight || 1) - 1);
      diag.lastNetworkActivityAt = Date.now();
      diag.lastMutationAt = Date.now();
    }, true);
  });

  const isOverlayNode = (node) => {
    if (!node || node.nodeType !== 1) return false;
    const id = node.id || '';
    if (id.startsWith('__open_browser')) return true;
    const cls = typeof node.className === 'string' ? node.className : '';
    if (cls.includes('ob-cursor') || cls.includes('ob-click-ripple') || cls.includes('ob-element-highlight')) return true;
    if (node.closest && node.closest('#__open_browser_cursor')) return true;
    return false;
  };

  const attachObserver = () => {
    const root = document.documentElement || document.body;
    if (!root || diag.__observerAttached) return;
    diag.__observerAttached = true;
    try {
      const obs = new MutationObserver((mutations) => {
        let relevant = false;
        for (const m of mutations) {
          if (isOverlayNode(m.target)) continue;
          if (m.type === 'childList') {
            const addedNonOverlay = Array.from(m.addedNodes || []).some((n) => !isOverlayNode(n));
            const removedNonOverlay = Array.from(m.removedNodes || []).some((n) => !isOverlayNode(n));
            if (!addedNonOverlay && !removedNonOverlay) continue;
          }
          relevant = true;
          break;
        }
        if (relevant) {
          diag.mutationCount = (diag.mutationCount || 0) + 1;
          diag.lastMutationAt = Date.now();
        }
      });
      obs.observe(root, { childList: true, subtree: true, characterData: true, attributes: true });
    } catch (_) {}
  };

  if (document.documentElement) {
    attachObserver();
  } else {
    window.addEventListener('DOMContentLoaded', attachObserver, { once: true });
  }
})();
"#;

#[derive(Clone, Debug)]
pub struct Tab {
  pub id: String,
  pub label: String,
  pub title: String,
  pub url: String,
  pub requested_url: Option<String>,
  pub error_code: Option<String>,
  pub http_status: Option<u16>,
  pub loading: bool,
  pub internal_new_tab: bool,
  pub visible: bool,
}

impl Tab {
  pub fn to_state(&self) -> TabState {
    TabState {
      id: self.id.clone(),
      title: self.title.clone(),
      url: self.url.clone(),
      loading: self.loading,
      can_go_back: true,
      can_go_forward: true,
      requested_url: self.requested_url.clone(),
      error_code: self.error_code.clone(),
      http_status: self.http_status,
    }
  }
}

#[derive(Clone, Debug)]
pub struct Space {
  pub id: String,
  pub name: String,
  pub kind: String,
  pub tabs: Vec<Tab>,
  pub active_tab_id: Option<String>,
  pub busy_count: u32,
  pub ignore_cert_errors: bool,
  pub data_dir: PathBuf,
}

impl Space {
  pub fn to_state(&self) -> SpaceState {
    let active_tab = self.tabs.iter().find(|t| Some(&t.id) == self.active_tab_id.as_ref());
    let status = if self.busy_count > 0 {
      "working".to_string()
    } else if active_tab.map_or(false, |t| t.loading) {
      "loading".to_string()
    } else {
      "ready".to_string()
    };

    SpaceState {
      id: self.id.clone(),
      name: self.name.clone(),
      kind: self.kind.clone(),
      status,
      active_tab_id: self.active_tab_id.clone(),
      ignore_cert_errors: self.ignore_cert_errors,
      tabs: self.tabs.iter().map(|t| t.to_state()).collect(),
    }
  }
}

pub fn normalize_navigation_target(input: &str, allow_file: bool) -> Result<String, String> {
  let value = input.trim();
  if value.is_empty() {
    return Err("La dirección está vacía.".into());
  }
  if value == "about:blank" {
    return Ok("about:blank".into());
  }

  let mut candidate = value.to_string();
  if !candidate.contains("://") {
    let is_localhost_or_ip = candidate.starts_with("localhost")
      || candidate.split(':').next().map_or(false, |h| {
        h.split('.').count() == 4 && h.chars().all(|c| c.is_ascii_digit() || c == '.')
      });

    if is_localhost_or_ip {
      candidate = format!("http://{}", candidate);
    } else {
      candidate = format!("https://{}", candidate);
    }
  }

  let parsed = Url::parse(&candidate).map_err(|_| "La dirección no es una URL válida.".to_string())?;
  let scheme = parsed.scheme();
  let allowed = if allow_file {
    scheme == "http" || scheme == "https" || scheme == "file"
  } else {
    scheme == "http" || scheme == "https"
  };

  if !allowed {
    return Err(format!("Protocolo no permitido: {}", scheme));
  }

  Ok(parsed.to_string())
}

pub struct SpaceManager {
  pub spaces: HashMap<String, Space>,
  pub active_space_id: Option<String>,
  pub app_handle: AppHandle,
  #[cfg(target_os = "linux")]
  pub has_active_flag: Arc<std::sync::atomic::AtomicBool>,
  #[cfg(target_os = "linux")]
  pub gtk_hook_installed: Arc<std::sync::atomic::AtomicBool>,
}

pub type SpaceManagerHandle = Arc<Mutex<SpaceManager>>;

impl SpaceManager {
  pub fn new(app_handle: AppHandle) -> Self {
    Self {
      spaces: HashMap::new(),
      active_space_id: None,
      app_handle,
      #[cfg(target_os = "linux")]
      has_active_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
      #[cfg(target_os = "linux")]
      gtk_hook_installed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
  }

  pub fn get_state(&self) -> BrowserState {
    BrowserState {
      active_space_id: self.active_space_id.clone(),
      spaces: self.spaces.values().map(|s| s.to_state()).collect(),
    }
  }

  pub fn emit_state(&self) {
    let state = self.get_state();
    let _ = self.app_handle.emit("browser:state", &state);
  }

  pub fn layout(&self) {
    let window = match self.app_handle.get_window("main") {
      Some(w) => w,
      None => return,
    };

    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1380, 900));
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    let content_height = (logical.height - TOOLBAR_HEIGHT).max(0.0) as i32;

    let has_active_tab = self.active_space_id.as_ref().map_or(false, |s_id| {
      self.spaces.get(s_id).map_or(false, |s| s.active_tab_id.is_some())
    });

    if let Some(main_wv) = self.app_handle.get_webview("main") {
      if has_active_tab {
        let _ = main_wv.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = main_wv.set_size(LogicalSize::new(logical.width, TOOLBAR_HEIGHT));
      } else {
        let _ = main_wv.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = main_wv.set_size(LogicalSize::new(logical.width, logical.height));
      }
    }

    for space in self.spaces.values() {
      for tab in &space.tabs {
        let is_active = self.active_space_id.as_deref() == Some(&space.id)
          && space.active_tab_id.as_deref() == Some(&tab.id);

        if let Some(wv) = self.app_handle.get_webview(&tab.label) {
          if is_active {
            let _ = wv.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
            let _ = wv.set_size(LogicalSize::new(logical.width, content_height as f64));
            let _ = wv.show();
          } else {
            let _ = wv.hide();
          }
        }
      }
    }

    #[cfg(target_os = "linux")]
    {
      use gtk::prelude::*;
      use std::sync::atomic::Ordering;

      self.has_active_flag.store(has_active_tab, Ordering::SeqCst);
      let flag = self.has_active_flag.clone();
      let installed = self.gtk_hook_installed.clone();

      if let Some(main_wv) = self.app_handle.get_webview("main") {
        let _ = main_wv.with_webview(move |platform_wv| {
          let widget = platform_wv.inner();
          if let Some(parent) = widget.parent() {
            if let Ok(vbox) = parent.dynamic_cast::<gtk::Box>() {
              if installed.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                let flag_cb = flag.clone();
                vbox.connect_size_allocate(move |vbox, total_alloc| {
                  let has_active = flag_cb.load(Ordering::SeqCst);
                  let children = vbox.children();
                  if children.len() > 1 && has_active {
                    let toolbar_h = TOOLBAR_HEIGHT as i32;
                    let alloc0 = gtk::Allocation::new(
                      total_alloc.x(),
                      total_alloc.y(),
                      total_alloc.width(),
                      toolbar_h,
                    );
                    children[0].size_allocate(&alloc0);

                    let content_y = total_alloc.y() + toolbar_h;
                    let content_h = (total_alloc.height() - toolbar_h).max(0);

                    for child in children.iter().skip(1) {
                      if child.is_visible() {
                        let alloc_c = gtk::Allocation::new(
                          total_alloc.x(),
                          content_y,
                          total_alloc.width(),
                          content_h,
                        );
                        child.size_allocate(&alloc_c);
                      }
                    }
                  }
                });
              }
              vbox.queue_resize();
            }
          }
        });
      }

      for space in self.spaces.values() {
        for tab in &space.tabs {
          let is_active = self.active_space_id.as_deref() == Some(&space.id)
            && space.active_tab_id.as_deref() == Some(&tab.id);

          if let Some(wv) = self.app_handle.get_webview(&tab.label) {
            let _ = wv.with_webview(move |platform_wv| {
              let widget = platform_wv.inner();
              if is_active {
                widget.show();
              } else {
                widget.hide();
              }
              widget.queue_resize();
            });
          }
        }
      }
    }
  }

  pub async fn create_space(
    manager_arc: &SpaceManagerHandle,
    name: Option<String>,
  ) -> Result<SpaceState, String> {
    let (space, _tab, _nav) = Self::create_space_with_options(manager_arc, name, None, false).await?;
    Ok(space)
  }

  pub async fn create_space_with_options(
    manager_arc: &SpaceManagerHandle,
    name: Option<String>,
    target_url: Option<String>,
    ignore_cert_errors: bool,
  ) -> Result<(SpaceState, TabState, Option<NavigationStatus>), String> {
    let space_id = Uuid::new_v4().to_string();
    let space_name = name
      .map(|n| n.trim().to_string())
      .filter(|n| !n.is_empty())
      .unwrap_or_else(|| "Agent space".to_string());

    let data_dir = std::env::temp_dir()
      .join("feather-open-browser-spaces")
      .join(&space_id);
    let _ = std::fs::create_dir_all(&data_dir);

    {
      let mut mgr = manager_arc.lock().await;
      let space = Space {
        id: space_id.clone(),
        name: space_name,
        kind: "agent".to_string(),
        tabs: Vec::new(),
        active_tab_id: None,
        busy_count: 0,
        ignore_cert_errors,
        data_dir,
      };
      mgr.spaces.insert(space_id.clone(), space);
      if mgr.active_space_id.is_none() {
        mgr.active_space_id = Some(space_id.clone());
      }
    }

    let (tab_state, nav_status) = Self::create_tab_with_status(
      manager_arc,
      &space_id,
      target_url.or_else(|| Some("about:blank".to_string())),
      true,
    )
    .await?;

    let mgr = manager_arc.lock().await;
    let space = mgr.spaces.get(&space_id).ok_or("No existe el Space recién creado")?;
    Ok((space.to_state(), tab_state, nav_status))
  }

  pub async fn activate_space(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
  ) -> Result<(), String> {
    {
      let mut mgr = manager_arc.lock().await;
      if !mgr.spaces.contains_key(space_id) {
        return Err("No existe ese Space.".into());
      }
      mgr.active_space_id = Some(space_id.to_string());
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn close_space(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
  ) -> Result<(), String> {
    let mut mgr = manager_arc.lock().await;
    if let Some(space) = mgr.spaces.remove(space_id) {
      for tab in space.tabs {
        if let Some(wv) = mgr.app_handle.get_webview(&tab.label) {
          let _ = wv.close();
        }
      }
      let _ = std::fs::remove_dir_all(&space.data_dir);
      if mgr.active_space_id.as_deref() == Some(space_id) {
        mgr.active_space_id = mgr.spaces.keys().next().cloned();
      }
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn create_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    target_url: Option<String>,
    activate: bool,
  ) -> Result<TabState, String> {
    let (tab_state, _nav) =
      Self::create_tab_with_status(manager_arc, space_id, target_url, activate).await?;
    Ok(tab_state)
  }

  pub async fn create_tab_with_status(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    target_url: Option<String>,
    activate: bool,
  ) -> Result<(TabState, Option<NavigationStatus>), String> {
    let (app_handle, tab_id, tab_label, url_str, is_about_blank, ignore_cert_errors, space_data_dir) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let raw_url = target_url.unwrap_or_else(|| "about:blank".to_string());
      let normalized = normalize_navigation_target(&raw_url, true)?;
      let is_blank = normalized == "about:blank";
      let t_id = Uuid::new_v4().to_string();
      let t_label = format!("wv_{}", t_id.replace('-', ""));
      (
        mgr.app_handle.clone(),
        t_id,
        t_label,
        normalized,
        is_blank,
        space.ignore_cert_errors,
        space.data_dir.clone(),
      )
    };

    let window = app_handle
      .get_window("main")
      .ok_or("Ventana principal no encontrada")?;

    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1380, 900));
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    let content_height = (logical.height - TOOLBAR_HEIGHT).max(0.0);

    let webview_url = if is_about_blank {
      WebviewUrl::App("newtab.html".into())
    } else {
      let parsed = Url::parse(&url_str).map_err(|e| e.to_string())?;
      WebviewUrl::External(parsed)
    };

    let mgr_weak = Arc::downgrade(manager_arc);
    let t_id_for_title = tab_id.clone();
    let mgr_weak_title = mgr_weak.clone();

    let t_id_for_load = tab_id.clone();
    let mgr_weak_load = mgr_weak.clone();

    let mut builder = WebviewBuilder::new(&tab_label, webview_url)
      .auto_resize()
      .data_directory(space_data_dir)
      .initialization_script(INIT_DIAGNOSTICS_SCRIPT)
      .on_document_title_changed(move |_wv, title| {
        if let Some(mgr_arc) = mgr_weak_title.upgrade() {
          let t_id = t_id_for_title.clone();
          tauri::async_runtime::spawn(async move {
            let mut mgr = mgr_arc.lock().await;
            mgr.on_tab_title_changed(&t_id, title);
          });
        }
      })
      .on_page_load(move |_wv, payload| {
        if let Some(mgr_arc) = mgr_weak_load.upgrade() {
          let t_id = t_id_for_load.clone();
          let page_url = payload.url().to_string();
          let is_loading = matches!(payload.event(), tauri::webview::PageLoadEvent::Started);
          tauri::async_runtime::spawn(async move {
            let mut mgr = mgr_arc.lock().await;
            mgr.on_tab_page_load(&t_id, page_url, is_loading);
          });
        }
      });

    if ignore_cert_errors {
      builder = builder.additional_browser_args(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --ignore-certificate-errors --allow-running-insecure-content",
      );
    }

    let wv = window
      .add_child(
        builder,
        LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
        LogicalSize::new(logical.width, content_height),
      )
      .map_err(|e| format!("Error creando webview de pestaña: {}", e))?;

    #[cfg(target_os = "linux")]
    if ignore_cert_errors {
      let _ = wv.with_webview(move |platform_wv| {
        #[allow(deprecated)]
        use webkit2gtk::{TLSErrorsPolicy, WebContextExt, WebViewExt, WebsiteDataManagerExt};
        if let Some(ctx) = platform_wv.inner().context() {
          #[allow(deprecated)]
          ctx.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
          if let Some(wdm) = ctx.website_data_manager() {
            wdm.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
          }
        }
      });
    }

    let tab = Tab {
      id: tab_id.clone(),
      label: tab_label,
      title: if is_about_blank { "Nueva pestaña".to_string() } else { "Cargando...".to_string() },
      url: if is_about_blank { "about:blank".to_string() } else { url_str.clone() },
      requested_url: if is_about_blank { None } else { Some(url_str.clone()) },
      error_code: None,
      http_status: None,
      loading: !is_about_blank,
      internal_new_tab: is_about_blank,
      visible: false,
    };

    {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        space.tabs.push(tab);
        if activate || space.active_tab_id.is_none() {
          space.active_tab_id = Some(tab_id.clone());
        }
      }
      mgr.layout();
      mgr.emit_state();
    }

    let nav_status = if !is_about_blank {
      Some(
        Self::wait_for_navigation_settlement(manager_arc, space_id, &tab_id, &wv, &url_str).await,
      )
    } else {
      None
    };

    let state = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      tab.to_state()
    };

    Ok((state, nav_status))
  }

  pub async fn activate_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    {
      let mut mgr = manager_arc.lock().await;
      let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;
      if !space.tabs.iter().any(|t| t.id == tab_id) {
        return Err("No existe esa pestaña.".into());
      }
      space.active_tab_id = Some(tab_id.to_string());
      mgr.active_space_id = Some(space_id.to_string());
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn close_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let mut mgr = manager_arc.lock().await;
    let app_handle = mgr.app_handle.clone();
    let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;

    if space.tabs.len() <= 1 {
      if let Some(tab) = space.tabs.first_mut() {
        tab.url = "about:blank".to_string();
        tab.requested_url = None;
        tab.error_code = None;
        tab.http_status = None;
        tab.title = "Nueva pestaña".to_string();
        tab.internal_new_tab = true;
        let label = tab.label.clone();
        if let Some(wv) = app_handle.get_webview(&label) {
          let _ = wv.navigate(
            Url::parse("tauri://localhost/newtab.html")
              .unwrap_or_else(|_| Url::parse("about:blank").unwrap()),
          );
        }
        mgr.emit_state();
      }
      return Ok(());
    }

    let tab_index = space.tabs.iter().position(|t| t.id == tab_id);
    if let Some(idx) = tab_index {
      let tab = space.tabs.remove(idx);
      if let Some(wv) = app_handle.get_webview(&tab.label) {
        let _ = wv.close();
      }
      if space.active_tab_id.as_deref() == Some(tab_id) {
        space.active_tab_id = space.tabs.first().map(|t| t.id.clone());
      }
      mgr.layout();
      mgr.emit_state();
    }

    Ok(())
  }

  pub async fn navigate(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    target: &str,
  ) -> Result<TabState, String> {
    let (tab_state, _nav) =
      Self::navigate_with_status(manager_arc, space_id, tab_id, target).await?;
    Ok(tab_state)
  }

  pub async fn navigate_with_status(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    target: &str,
  ) -> Result<(TabState, NavigationStatus), String> {
    let normalized = normalize_navigation_target(target, true)?;
    let is_about_blank = normalized == "about:blank";

    let (app_handle, label) = {
      let mut mgr = manager_arc.lock().await;
      let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter_mut().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;

      tab.url = if is_about_blank { "about:blank".to_string() } else { normalized.clone() };
      tab.requested_url = if is_about_blank { None } else { Some(normalized.clone()) };
      tab.error_code = None;
      tab.http_status = None;
      tab.internal_new_tab = is_about_blank;
      tab.loading = !is_about_blank;
      if is_about_blank {
        tab.title = "Nueva pestaña".to_string();
      } else {
        tab.title = "Cargando...".to_string();
      }
      let tab_label = tab.label.clone();
      let app_handle = mgr.app_handle.clone();
      mgr.emit_state();
      (app_handle, tab_label)
    };

    let wv = app_handle
      .get_webview(&label)
      .ok_or("Webview de la pestaña no disponible")?;

    if is_about_blank {
      let _ = wv.navigate(
        Url::parse("tauri://localhost/newtab.html")
          .unwrap_or_else(|_| Url::parse("about:blank").unwrap()),
      );
      let state = {
        let mgr = manager_arc.lock().await;
        let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
        let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
        tab.to_state()
      };
      let nav = NavigationStatus {
        ok: true,
        requested_url: "about:blank".to_string(),
        url: "about:blank".to_string(),
        title: "Nueva pestaña".to_string(),
        error_code: None,
        http_status: None,
      };
      return Ok((state, nav));
    }

    if let Ok(parsed) = Url::parse(&normalized) {
      let _ = eval_js_with_timeout(
        &wv,
        "try { window.__openBrowserLeavingPage = true; } catch (_) {}",
        300,
      )
      .await;
      let _ = wv.navigate(parsed);
    }

    let nav_status =
      Self::wait_for_navigation_settlement(manager_arc, space_id, tab_id, &wv, &normalized).await;

    let state = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      tab.to_state()
    };

    Ok((state, nav_status))
  }

  async fn wait_for_navigation_settlement(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    wv: &tauri::Webview,
    requested_url: &str,
  ) -> NavigationStatus {
    let start = tokio::time::Instant::now();
    let max_wait = tokio::time::Duration::from_millis(8000);

    // Give the webview a brief moment to start loading
    tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;

    let probe_script = r#"
      JSON.stringify((function() {
        try {
          const href = String(location.href || '');
          const title = String(document.title || '');
          const readyState = String(document.readyState || '');
          let httpStatus = null;
          try {
            const navEntries = performance.getEntriesByType('navigation');
            if (navEntries && navEntries.length > 0 && typeof navEntries[0].responseStatus === 'number' && navEntries[0].responseStatus > 0) {
              httpStatus = navEntries[0].responseStatus;
            }
          } catch (_) {}

          const bodyText = (document.body ? (document.body.innerText || '') : (document.documentElement ? (document.documentElement.innerText || '') : '')).slice(0, 4000);
          let errorCode = null;

          const errNode = document.querySelector('.error-code, #error-code, [id*="error-code"]');
          if (errNode) {
            const t = (errNode.innerText || errNode.textContent || '').trim();
            if (t) errorCode = t;
          }
          if (!errorCode) {
            const match = bodyText.match(/\b((?:NET::|net::)?ERR_[A-Z0-9_]+)\b/);
            if (match) errorCode = match[1];
          }
          if (!errorCode && window.loadTimeData && window.loadTimeData.data_ && window.loadTimeData.data_.errorCode) {
            errorCode = String(window.loadTimeData.data_.errorCode);
          }

          const isErrorUrl = href.startsWith('chrome-error://') ||
            href.startsWith('edge-error://') ||
            href.startsWith('about:neterror') ||
            href.startsWith('webkit-error:');

          const isCertPage = Boolean(
            window.certificateErrorPageController ||
            document.querySelector('#proceed-link, #details-button, .ssl-error, .interstitial-wrapper')
          );

          if ((isErrorUrl || isCertPage) && !errorCode) {
            errorCode = isCertPage ? 'NET::ERR_CERT_INVALID' : 'net::ERR_FAILED';
          }

          return {
            href,
            title,
            readyState,
            httpStatus,
            errorCode,
            isErrorUrl,
            isCertPage,
            leavingOldPage: Boolean(window.__openBrowserLeavingPage),
            bodyLen: bodyText.trim().length
          };
        } catch (e) {
          return { error: String(e) };
        }
      })())
    "#;

    let mut last_probe: Option<serde_json::Value> = None;

    while start.elapsed() < max_wait {
      let is_loading = {
        let mgr = manager_arc.lock().await;
        mgr
          .spaces
          .get(space_id)
          .and_then(|s| s.tabs.iter().find(|t| t.id == tab_id))
          .map(|t| t.loading)
          .unwrap_or(false)
      };

      if let Ok(val) = eval_json_with_timeout(wv, probe_script, 1200).await {
        let href = val.get("href").and_then(|v| v.as_str()).unwrap_or("");
        let ready = val.get("readyState").and_then(|v| v.as_str()).unwrap_or("");
        let is_err_url = val.get("isErrorUrl").and_then(|v| v.as_bool()).unwrap_or(false);
        let has_err_code = val.get("errorCode").and_then(|v| v.as_str()).is_some();
        let leaving_old_page = val.get("leavingOldPage").and_then(|v| v.as_bool()).unwrap_or(false);

        last_probe = Some(val.clone());

        if is_err_url || has_err_code {
          break;
        }

        let is_still_initial_blank = href.is_empty()
          || href == "about:blank"
          || href.contains("newtab.html");
        let is_waiting_for_transition = is_still_initial_blank || leaving_old_page;

        if !is_waiting_for_transition && (!is_loading || ready == "complete" || ready == "interactive") {
          break;
        }

        // If page load finished (or never started after 2.6s) and we never left the initial/previous page
        if is_waiting_for_transition && !is_loading && start.elapsed() >= tokio::time::Duration::from_millis(1200) {
          break;
        }
        if is_waiting_for_transition && start.elapsed() >= tokio::time::Duration::from_millis(2600) {
          break;
        }
      }

      tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
    }

    let _ = eval_js_with_timeout(wv, INIT_DIAGNOSTICS_SCRIPT, 800).await;

    let probe = last_probe.unwrap_or_else(|| serde_json::json!({}));
    let raw_href = probe.get("href").and_then(|v| v.as_str()).unwrap_or("");
    let raw_title = probe.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let is_error_url = probe.get("isErrorUrl").and_then(|v| v.as_bool()).unwrap_or(false);
    let leaving_old_page = probe.get("leavingOldPage").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut error_code = probe
      .get("errorCode")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());
    let http_status = probe
      .get("httpStatus")
      .and_then(|v| v.as_u64())
      .map(|n| n as u16);

    let stuck_on_blank = raw_href.is_empty()
      || raw_href == "about:blank"
      || raw_href.contains("newtab.html")
      || leaving_old_page;

    if stuck_on_blank && requested_url != "about:blank" && error_code.is_none() {
      error_code = Some("net::ERR_CONNECTION_FAILED".to_string());
      // Render an informative error view in the tab so snapshot and screenshot reflect the failed navigation
      let render_err_script = format!(
        r#"
        (function() {{
          try {{
            document.title = 'Error de conexión';
            const body = document.body || document.documentElement;
            if (body) {{
              while (body.firstChild) body.removeChild(body.firstChild);
              const wrap = document.createElement('main');
              wrap.style.cssText = 'font-family: system-ui, sans-serif; padding: 32px; color: #e6edf3; background: #101318; min-height: 100vh;';
              const h1 = document.createElement('h1');
              h1.textContent = 'No se pudo acceder a este sitio web';
              const pUrl = document.createElement('p');
              pUrl.textContent = 'URL solicitada: ' + {:?};
              const pCode = document.createElement('p');
              pCode.className = 'error-code';
              pCode.textContent = 'net::ERR_CONNECTION_FAILED';
              const btn = document.createElement('button');
              btn.textContent = 'Reintentar';
              btn.onclick = function() {{ location.href = {:?}; }};
              wrap.appendChild(h1);
              wrap.appendChild(pUrl);
              wrap.appendChild(pCode);
              wrap.appendChild(btn);
              body.appendChild(wrap);
            }}
          }} catch (_) {{}}
        }})()
        "#,
        requested_url, requested_url
      );
      let _ = eval_js_with_timeout(wv, &render_err_script, 800).await;
    }

    let ok = error_code.is_none()
      && !is_error_url
      && !stuck_on_blank
      && http_status.map_or(true, |s| s < 400);

    let effective_url = if is_error_url || stuck_on_blank {
      requested_url.to_string()
    } else if !raw_href.is_empty() {
      raw_href.to_string()
    } else {
      requested_url.to_string()
    };

    let effective_title = if !stuck_on_blank && !raw_title.trim().is_empty() && raw_title != "Nueva pestaña" {
      raw_title.trim().to_string()
    } else if let Some(ref code) = error_code {
      format!("Error de navegación ({})", code)
    } else {
      effective_url.clone()
    };

    {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        if let Some(tab) = space.tabs.iter_mut().find(|t| t.id == tab_id) {
          tab.loading = false;
          tab.url = effective_url.clone();
          tab.title = effective_title.clone();
          tab.error_code = error_code.clone();
          tab.http_status = http_status;
        }
      }
      mgr.emit_state();
    }

    NavigationStatus {
      ok,
      requested_url: requested_url.to_string(),
      url: effective_url,
      title: effective_title,
      error_code,
      http_status,
    }
  }

  pub async fn go_back(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.eval("window.history.back()");
    }
    Ok(())
  }

  pub async fn go_forward(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.eval("window.history.forward()");
    }
    Ok(())
  }

  pub async fn reload(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.reload();
    }
    Ok(())
  }

  pub fn on_tab_title_changed(&mut self, tab_id: &str, title: String) {
    let trimmed = title.trim();
    if trimmed.is_empty() {
      return;
    }
    for space in self.spaces.values_mut() {
      for tab in &mut space.tabs {
        if tab.id == tab_id {
          if !tab.internal_new_tab {
            tab.title = trimmed.to_string();
          }
          self.emit_state();
          return;
        }
      }
    }
  }

  pub fn on_tab_page_load(&mut self, tab_id: &str, url: String, loading: bool) {
    for space in self.spaces.values_mut() {
      for tab in &mut space.tabs {
        if tab.id == tab_id {
          tab.loading = loading;
          if !loading && !tab.internal_new_tab {
            let is_err_or_blank = url.starts_with("chrome-error://")
              || url.starts_with("edge-error://")
              || url == "about:blank"
              || url.contains("newtab.html");
            if !is_err_or_blank {
              tab.url = url;
            }
          }
          self.emit_state();
          return;
        }
      }
    }
  }

  pub async fn snapshot(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<serde_json::Value, String> {
    Self::snapshot_with_options(manager_arc, space_id, tab_id, SnapshotOptions::default()).await
  }

  pub async fn snapshot_with_options(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    options: SnapshotOptions,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label, fallback_url, fallback_title, stored_error_code, stored_http_status) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (
        mgr.app_handle.clone(),
        tab.label.clone(),
        tab.url.clone(),
        tab.title.clone(),
        tab.error_code.clone(),
        tab.http_status,
      )
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview de la pestaña no disponible")?;

    let _ = eval_js_with_timeout(&wv, INIT_DIAGNOSTICS_SCRIPT, 800).await;

    let opts_json = serde_json::to_string(&serde_json::json!({
      "selector": options.selector,
      "role": options.role,
      "offset": options.offset.unwrap_or(0),
      "limit": options.limit.unwrap_or(180).clamp(1, 500),
      "includeTables": options.include_tables.unwrap_or(true),
      "maxTableRows": options.max_table_rows.unwrap_or(50).clamp(1, 500),
      "tableRowOffset": options.table_row_offset.unwrap_or(0),
    }))
    .unwrap_or_else(|_| "{}".to_string());

    let script = format!(
      r#"
      JSON.stringify((function() {{
        const opts = {};
        const refs = new Map();
        const elToRef = new WeakMap();
        window.__openBrowserAgentRefs = refs;
        let nextRef = 1;

        let root = document;
        if (opts.selector) {{
          root = document.querySelector(opts.selector);
          if (!root) {{
            return {{
              ok: false,
              error: 'No se encontró ningún elemento para el selector: ' + opts.selector
            }};
          }}
        }}

        const visible = (el) => {{
          if (!el || typeof el.getBoundingClientRect !== 'function') return false;
          const rect = el.getBoundingClientRect();
          const style = getComputedStyle(el);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
            style.visibility !== 'hidden' && style.opacity !== '0';
        }};

        const cleanText = (str, maxLen = 240) =>
          String(str || '').replace(/\s+/g, ' ').trim().slice(0, maxLen);

        const text = (el) => cleanText(
          el.getAttribute('aria-label') || el.innerText || el.textContent ||
          el.value || el.getAttribute('placeholder') || el.getAttribute('alt') ||
          el.getAttribute('title') || ''
        );

        const role = (el) => {{
          if (el.getAttribute('role')) return el.getAttribute('role');
          const tag = el.tagName.toLowerCase();
          if (tag === 'a') return 'link';
          if (tag === 'button' || tag === 'summary') return 'button';
          if (tag === 'textarea') return 'textbox';
          if (tag === 'select') return 'combobox';
          if (tag === 'th') return 'columnheader';
          if (tag === 'input') {{
            const type = (el.getAttribute('type') || 'text').toLowerCase();
            if (type === 'checkbox') return 'checkbox';
            if (type === 'radio') return 'radio';
            if (type === 'submit' || type === 'button' || type === 'reset') return 'button';
            return 'textbox';
          }}
          if (el.hasAttribute('hx-get') || el.hasAttribute('hx-post') || el.hasAttribute('hx-put') || el.hasAttribute('hx-delete') || el.hasAttribute('onclick')) {{
            return 'button';
          }}
          return 'generic';
        }};

        const candidateSelector = [
          'a[href]',
          'button',
          'input:not([type="hidden"])',
          'textarea',
          'select',
          'summary',
          '[role="button"]',
          '[role="link"]',
          '[role="checkbox"]',
          '[role="radio"]',
          '[role="tab"]',
          '[role="menuitem"]',
          '[role="option"]',
          '[role="switch"]',
          '[role="columnheader"]',
          '[contenteditable="true"]',
          '[hx-get]',
          '[hx-post]',
          '[hx-put]',
          '[hx-delete]',
          'th[onclick]',
          'tr[onclick]'
        ].join(',');

        const allCandidates = Array.from(root.querySelectorAll(candidateSelector)).filter(visible);
        const allItems = [];

        for (const el of allCandidates) {{
          const ref = 'e' + nextRef++;
          refs.set(ref, el);
          elToRef.set(el, ref);
          const itemRole = role(el);
          const name = text(el) || '(sin nombre)';
          const href = el instanceof HTMLAnchorElement && el.href ? ' -> ' + el.href : '';
          let value = '';
          if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {{
            if (el.type === 'checkbox' || el.type === 'radio') {{
              value = ' checked=' + Boolean(el.checked);
            }} else {{
              value = ' value=' + JSON.stringify((el.value || '').slice(0, 100));
            }}
          }} else if (el instanceof HTMLSelectElement) {{
            const selectedText = el.options && el.selectedIndex >= 0 ? (el.options[el.selectedIndex]?.text || el.value) : el.value;
            value = ' value=' + JSON.stringify((selectedText || '').slice(0, 100));
          }}
          allItems.push({{
            role: itemRole,
            line: itemRole + ' ' + JSON.stringify(name) + ' [ref=' + ref + ']' + href + value
          }});
        }}

        const filteredItems = opts.role
          ? allItems.filter((item) => item.role.toLowerCase() === String(opts.role).toLowerCase())
          : allItems;

        const offset = Math.max(0, Number(opts.offset) || 0);
        const limit = Math.max(1, Number(opts.limit) || 180);
        const pagedRows = filteredItems.slice(offset, offset + limit).map((item) => item.line);

        // Extract structured tables (#6)
        const tables = [];
        if (opts.includeTables !== false) {{
          const tableSelector = 'table, [role="table"], [role="grid"]';
          const tableNodes = [];
          if (root.matches && root.matches(tableSelector) && visible(root)) {{
            tableNodes.push(root);
          }}
          tableNodes.push(...Array.from(root.querySelectorAll(tableSelector)).filter(visible));

          const formatCell = (cell) => {{
            const baseText = cleanText(cell.innerText || cell.textContent || '', 200);
            const cellRefs = [];
            if (elToRef.has(cell)) {{
              cellRefs.push(elToRef.get(cell));
            }}
            const interactiveChildren = cell.querySelectorAll(candidateSelector);
            for (const child of interactiveChildren) {{
              if (elToRef.has(child)) {{
                const r = elToRef.get(child);
                if (!cellRefs.includes(r)) cellRefs.push(r);
              }}
            }}
            if (cellRefs.length > 0) {{
              return (baseText ? baseText + ' ' : '') + '[' + cellRefs.map((r) => 'ref=' + r).join(', ') + ']';
            }}
            return baseText;
          }};

          const rowOffset = Math.max(0, Number(opts.tableRowOffset) || 0);
          const maxRows = Math.max(1, Number(opts.maxTableRows) || 50);

          for (const tbl of tableNodes.slice(0, 20)) {{
            let headers = [];
            const theadCells = Array.from(tbl.querySelectorAll('thead tr th, thead tr td, [role="columnheader"]')).filter(visible);
            let headerRowEl = null;

            if (theadCells.length > 0) {{
              headers = theadCells.map(formatCell);
            }} else {{
              const firstRow = tbl.querySelector('tr, [role="row"]');
              if (firstRow) {{
                const ths = Array.from(firstRow.querySelectorAll('th, [role="columnheader"]')).filter(visible);
                if (ths.length > 0) {{
                  headers = ths.map(formatCell);
                  headerRowEl = firstRow;
                }}
              }}
            }}

            const allTrs = Array.from(tbl.querySelectorAll('tbody tr, tr, [role="row"]'))
              .filter((tr) => visible(tr) && tr !== headerRowEl && !tr.closest('thead'));

            // Deduplicate nested or repeated tr references
            const uniqueTrs = [];
            const seenTr = new Set();
            for (const tr of allTrs) {{
              if (!seenTr.has(tr)) {{
                seenTr.add(tr);
                const cells = Array.from(tr.querySelectorAll(':scope > td, :scope > th, [role="cell"], [role="gridcell"]'));
                if (cells.length > 0) {{
                  uniqueTrs.push(cells.map(formatCell));
                }}
              }}
            }}

            const slicedRows = uniqueTrs.slice(rowOffset, rowOffset + maxRows);
            const caption = cleanText(
              (tbl.caption && tbl.caption.innerText) || tbl.getAttribute('aria-label') || tbl.id || '',
              120
            );

            if (headers.length > 0 || uniqueTrs.length > 0) {{
              tables.push({{
                ...(caption ? {{ caption }} : {{}}),
                headers,
                rows: slicedRows,
                totalRows: uniqueTrs.length,
                rowOffset,
                truncated: (rowOffset + slicedRows.length) < uniqueTrs.length
              }});
            }}
          }}
        }}

        const textBlocks = Array.from(root.querySelectorAll(
          'h1,h2,h3,h4,h5,h6,p,li,dt,dd,blockquote,summary,[role="alert"],[role="status"],.error-code'
        ))
          .filter((el) => visible(el) && !el.closest('table'))
          .slice(0, 120)
          .map((el) => el.tagName.toLowerCase() + ': ' + cleanText(el.innerText || el.textContent || '', 280))
          .filter((line) => line.length > 4);

        const diag = window.__openBrowserDiag || {{}};
        const consoleLogs = Array.isArray(diag.consoleLogs) ? diag.consoleLogs : [];
        const consoleErrors = consoleLogs.filter((l) => l.level === 'error').slice(-10);
        const netReqs = Array.isArray(diag.networkRequests) ? diag.networkRequests : [];
        let resourceCount = netReqs.length;
        try {{
          const perfRes = performance.getEntriesByType('resource');
          if (perfRes && perfRes.length > resourceCount) resourceCount = perfRes.length;
        }} catch (_) {{}}

        let httpStatus = null;
        try {{
          const navEntries = performance.getEntriesByType('navigation');
          if (navEntries && navEntries.length > 0 && typeof navEntries[0].responseStatus === 'number' && navEntries[0].responseStatus > 0) {{
            httpStatus = navEntries[0].responseStatus;
          }}
        }} catch (_) {{}}

        let errorCode = null;
        const errEl = document.querySelector('.error-code, #error-code');
        if (errEl) {{
          const t = cleanText(errEl.innerText || errEl.textContent || '', 120);
          if (t) errorCode = t;
        }}
        if (!errorCode && (location.href.startsWith('chrome-error://') || window.certificateErrorPageController)) {{
          const bodyStr = (document.body ? document.body.innerText : '') || '';
          const m = bodyStr.match(/\b((?:NET::|net::)?ERR_[A-Z0-9_]+)\b/);
          errorCode = m ? m[1] : 'NET::ERR_CERT_OR_NAVIGATION';
        }}

        return {{
          ok: true,
          title: document.title || '',
          url: location.href || '',
          httpStatus,
          errorCode,
          elements: pagedRows,
          totalElements: filteredItems.length,
          offset,
          limit,
          truncated: (offset + pagedRows.length) < filteredItems.length,
          tables,
          text: textBlocks,
          consoleErrors,
          networkRequestsCount: resourceCount,
          inFlightRequests: (diag.inFlightRequests || 0) + (diag.htmxInFlight || 0),
          domMutations: diag.mutationCount || 0
        }};
      }})())
      "#,
      opts_json
    );

    let parsed = match eval_json_with_timeout(&wv, &script, 5000).await {
      Ok(val) => val,
      Err(err) => {
        return Ok(serde_json::json!({
          "title": fallback_title,
          "url": fallback_url,
          "errorCode": stored_error_code.unwrap_or_else(|| "ERR_PAGE_UNRESPONSIVE".to_string()),
          "httpStatus": stored_http_status,
          "elements": [],
          "tables": [],
          "text": [format!("error: {}", err)],
          "truncated": false,
          "consoleErrors": [],
          "networkRequestsCount": 0
        }));
      }
    };

    if parsed.get("ok").and_then(|v| v.as_bool()) == Some(false) {
      let err_msg = parsed
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("Error generando snapshot");
      return Err(err_msg.to_string());
    }

    // Live sync title and url back to TabState (#11)
    let live_title = parsed.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let live_url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let live_err = parsed.get("errorCode").and_then(|v| v.as_str()).map(|s| s.to_string());
    let live_status = parsed.get("httpStatus").and_then(|v| v.as_u64()).map(|n| n as u16);

    let (effective_url, effective_title) = {
      let mut mgr = manager_arc.lock().await;
      let mut eff_url = fallback_url;
      let mut eff_title = fallback_title;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        if let Some(tab) = space.tabs.iter_mut().find(|t| t.id == tab_id) {
          if tab.internal_new_tab {
            eff_url = "about:blank".to_string();
            eff_title = "Nueva pestaña".to_string();
          } else {
            let is_internal_err = live_url.starts_with("chrome-error://")
              || live_url.starts_with("edge-error://")
              || live_url == "about:blank"
              || live_url.contains("newtab.html");
            if !live_url.is_empty() && !is_internal_err {
              tab.url = live_url.clone();
              eff_url = live_url;
            }
            if !live_title.is_empty() {
              tab.title = live_title.clone();
              eff_title = live_title;
            }
            if live_err.is_some() {
              tab.error_code = live_err.clone();
            }
            if live_status.is_some() {
              tab.http_status = live_status;
            }
          }
        }
      }
      mgr.emit_state();
      (eff_url, eff_title)
    };

    let mut result_obj = parsed;
    if let Some(map) = result_obj.as_object_mut() {
      map.remove("ok");
      map.insert("url".to_string(), serde_json::json!(effective_url));
      map.insert("title".to_string(), serde_json::json!(effective_title));
      if map.get("errorCode").and_then(|v| v.as_str()).is_none() {
        if let Some(code) = stored_error_code {
          map.insert("errorCode".to_string(), serde_json::json!(code));
        }
      }
    }

    Ok(result_obj)
  }

  pub async fn extract(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    options: ExtractOptions,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;

    let opts_json = serde_json::to_string(&serde_json::json!({
      "selector": options.selector,
      "limit": options.limit.unwrap_or(50).clamp(1, 500),
      "offset": options.offset.unwrap_or(0),
      "includeAttributes": options.include_attributes.unwrap_or(false),
    }))
    .unwrap_or_else(|_| "{}".to_string());

    let script = format!(
      r#"
      JSON.stringify((function() {{
        const opts = {};
        const cleanText = (str, maxLen = 500) =>
          String(str || '').replace(/\s+/g, ' ').trim().slice(0, maxLen);

        let nodes;
        try {{
          nodes = Array.from(document.querySelectorAll(opts.selector));
        }} catch (e) {{
          return {{ ok: false, error: 'Selector CSS inválido: ' + opts.selector }};
        }}

        const totalMatches = nodes.length;
        const offset = Math.max(0, Number(opts.offset) || 0);
        const limit = Math.max(1, Number(opts.limit) || 50);
        const sliced = nodes.slice(offset, offset + limit);

        const items = sliced.map((el, i) => {{
          const tag = el.tagName.toLowerCase();
          if (tag === 'table') {{
            const headers = Array.from(el.querySelectorAll('thead th, thead td'))
              .map((c) => cleanText(c.innerText || c.textContent, 200));
            const rows = Array.from(el.querySelectorAll('tbody tr, tr'))
              .filter((tr) => !tr.closest('thead'))
              .map((tr) => Array.from(tr.querySelectorAll(':scope > td, :scope > th')).map((c) => cleanText(c.innerText || c.textContent, 200)))
              .filter((r) => r.length > 0);
            return {{
              index: offset + i,
              tag,
              headers,
              rows: rows.slice(0, limit),
              totalRows: rows.length
            }};
          }}
          if (tag === 'tr') {{
            const cells = Array.from(el.querySelectorAll(':scope > td, :scope > th'))
              .map((c) => cleanText(c.innerText || c.textContent, 240));
            return {{
              index: offset + i,
              tag,
              cells
            }};
          }}

          const entry = {{
            index: offset + i,
            tag,
            text: cleanText(el.innerText || el.textContent || '', 600)
          }};
          if (el instanceof HTMLAnchorElement && el.href) entry.href = el.href;
          if ('value' in el && el.value !== undefined && el.value !== '') entry.value = String(el.value).slice(0, 200);
          if (opts.includeAttributes && el.attributes) {{
            const attrs = {{}};
            for (const attr of Array.from(el.attributes)) {{
              attrs[attr.name] = attr.value.slice(0, 200);
            }}
            entry.attributes = attrs;
          }}
          return entry;
        }});

        return {{
          ok: true,
          url: location.href,
          title: document.title,
          selector: opts.selector,
          totalMatches,
          offset,
          limit,
          truncated: (offset + items.length) < totalMatches,
          items
        }};
      }})())
      "#,
      opts_json
    );

    let mut val = eval_json_async(&wv, &script).await?;
    if val.get("ok").and_then(|v| v.as_bool()) == Some(false) {
      let err_msg = val
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("Error extrayendo nodos");
      return Err(err_msg.to_string());
    }
    if let Some(map) = val.as_object_mut() {
      map.remove("ok");
    }
    Ok(val)
  }

  pub async fn run_actions(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    actions: serde_json::Value,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        space.busy_count += 1;
      }
      mgr.emit_state();
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let actions_list = actions.as_array().cloned().unwrap_or_default();
    let wv = app_handle.get_webview(&label).ok_or("Webview de la pestaña no disponible")?;
    let fast = std::env::var("OPEN_BROWSER_FAST_ACTIONS").as_deref() == Ok("1");

    let mut results = Vec::new();
    let mut needs_settle = false;
    let mut action_error: Option<String> = None;

    for action in actions_list {
      let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
      match action_type {
        "click" | "fill" | "press" | "hover" | "scroll" | "proceed_unsafe" => {
          match execute_visual_action(&wv, &action, fast).await {
            Ok(res) => {
              results.push(res);
              if action_type == "click" || action_type == "fill" || action_type == "press" || action_type == "proceed_unsafe" {
                needs_settle = true;
              }
            }
            Err(err) => {
              action_error = Some(err);
              break;
            }
          }
        }

        "navigate" => {
          if let Some(target) = action.get("url").and_then(|v| v.as_str()) {
            match Self::navigate_with_status(manager_arc, space_id, tab_id, target).await {
              Ok((_tab, nav)) => {
                results.push(serde_json::json!({
                  "navigated": target,
                  "ok": nav.ok,
                  "url": nav.url,
                  "errorCode": nav.error_code,
                  "httpStatus": nav.http_status
                }));
                needs_settle = false;
              }
              Err(err) => {
                action_error = Some(err);
                break;
              }
            }
          }
        }

        "back" => {
          let _ = Self::go_back(manager_arc, space_id, tab_id).await;
          needs_settle = true;
          results.push(serde_json::json!({ "ok": true }));
        }

        "forward" => {
          let _ = Self::go_forward(manager_arc, space_id, tab_id).await;
          needs_settle = true;
          results.push(serde_json::json!({ "ok": true }));
        }

        "reload" => {
          let _ = Self::reload(manager_arc, space_id, tab_id).await;
          needs_settle = true;
          results.push(serde_json::json!({ "ok": true }));
        }

        "wait" => {
          let until = action.get("until").and_then(|v| v.as_str());
          let selector = action.get("selector").and_then(|v| v.as_str());
          let default_ms = if until.is_some() || selector.is_some() { 5000 } else { 500 };
          let ms = action
            .get("ms")
            .or_else(|| action.get("durationMs"))
            .or_else(|| action.get("timeoutMs"))
            .and_then(|v| v.as_u64())
            .unwrap_or(default_ms)
            .min(15000);

          let wait_res = wait_for_condition(&wv, ms, until, selector).await;
          needs_settle = false;
          results.push(wait_res);
        }

        _ => {}
      }
    }

    if needs_settle && action_error.is_none() {
      settle_after_interaction(&wv).await;
    }

    {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        if space.busy_count > 0 {
          space.busy_count -= 1;
        }
      }
      mgr.emit_state();
    }

    if let Some(err) = action_error {
      return Err(err);
    }

    let snapshot = Self::snapshot(manager_arc, space_id, tab_id)
      .await
      .unwrap_or_else(|_| serde_json::json!({}));

    Ok(serde_json::json!({
      "results": results,
      "snapshot": snapshot,
    }))
  }

  pub async fn get_html(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    options: GetHtmlOptions,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;

    let opts_json = serde_json::to_string(&serde_json::json!({
      "ref": options.ref_id,
      "selector": options.selector,
      "maxChars": options.max_chars.unwrap_or(20000).clamp(100, 200000),
      "offset": options.offset.unwrap_or(0),
      "textOnly": options.text_only.unwrap_or(false),
    }))
    .unwrap_or_else(|_| "{}".to_string());

    let script = format!(
      r#"
      JSON.stringify((function() {{
        const opts = {};
        let target = null;

        if (opts.ref) {{
          target = window.__openBrowserAgentRefs?.get(opts.ref) || null;
          if (!target || !target.isConnected) {{
            return {{
              ok: false,
              error: "El ref '" + opts.ref + "' expiró o el elemento ya no está en el DOM. Vuelve a tomar un browser_snapshot."
            }};
          }}
        }} else if (opts.selector) {{
          try {{
            target = document.querySelector(opts.selector);
          }} catch (e) {{
            return {{
              ok: false,
              error: "Selector CSS inválido: " + opts.selector
            }};
          }}
          if (!target) {{
            return {{
              ok: false,
              error: "No se encontró ningún elemento para el selector: " + opts.selector
            }};
          }}
        }} else {{
          target = document.documentElement || document.body;
        }}

        if (!target) {{
          return {{
            ok: false,
            error: "El documento aún no tiene contenido DOM disponible."
          }};
        }}

        let rawContent = '';
        if (opts.textOnly) {{
          rawContent = String(target.innerText || target.textContent || '');
        }} else {{
          rawContent = String(target.outerHTML || '');
        }}

        rawContent = rawContent.replace(/\r\n/g, '\n');
        const totalChars = rawContent.length;
        const offset = Math.max(0, Number(opts.offset) || 0);
        const maxChars = Math.max(100, Number(opts.maxChars) || 20000);
        const slice = rawContent.slice(offset, offset + maxChars);

        return {{
          ok: true,
          url: location.href,
          title: document.title,
          ref: opts.ref || null,
          selector: opts.selector || null,
          textOnly: Boolean(opts.textOnly),
          totalChars,
          offset,
          maxChars,
          truncated: (offset + slice.length) < totalChars,
          html: slice
        }};
      }})())
      "#,
      opts_json
    );

    let mut val = eval_json_async(&wv, &script).await?;
    if val.get("ok").and_then(|v| v.as_bool()) == Some(false) {
      let err_msg = val
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("No se pudo obtener el HTML solicitado.");
      return Err(err_msg.to_string());
    }

    if let Some(map) = val.as_object_mut() {
      map.remove("ok");
    }

    Ok(val)
  }

  pub async fn get_console_logs(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    limit: Option<usize>,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;
    let max = limit.unwrap_or(100).clamp(1, 200);
    let script = format!(
      r#"
      JSON.stringify((function() {{
        const logs = window.__openBrowserDiag?.consoleLogs || [];
        return {{ logs: logs.slice(-{}) }};
      }})())
      "#,
      max
    );
    match eval_json_with_timeout(&wv, &script, 3000).await {
      Ok(val) => Ok(val),
      Err(_) => Ok(serde_json::json!({ "logs": [] })),
    }
  }

  pub async fn get_network_requests(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    limit: Option<usize>,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;
    let max = limit.unwrap_or(100).clamp(1, 200);
    let script = format!(
      r#"
      JSON.stringify((function() {{
        const diag = window.__openBrowserDiag || {{}};
        const tracked = Array.isArray(diag.networkRequests) ? diag.networkRequests.slice() : [];
        const seenUrls = new Set(tracked.map((r) => r.url));
        try {{
          const navEntries = performance.getEntriesByType('navigation') || [];
          for (const n of navEntries) {{
            if (n.name && !seenUrls.has(n.name)) {{
              tracked.unshift({{
                id: 0,
                method: 'GET',
                url: n.name,
                resourceType: 'document',
                status: n.responseStatus || 200,
                ok: !n.responseStatus || (n.responseStatus >= 200 && n.responseStatus < 400),
                durationMs: Math.round(n.duration || 0),
                timestamp: Math.round(performance.timeOrigin + (n.startTime || 0))
              }});
              seenUrls.add(n.name);
            }}
          }}
          const resEntries = performance.getEntriesByType('resource') || [];
          for (const r of resEntries.slice(-100)) {{
            if (r.name && !seenUrls.has(r.name)) {{
              tracked.push({{
                url: r.name,
                method: 'GET',
                resourceType: r.initiatorType || 'resource',
                status: r.responseStatus || 200,
                durationMs: Math.round(r.duration || 0),
                timestamp: Math.round(performance.timeOrigin + (r.startTime || 0))
              }});
              seenUrls.add(r.name);
            }}
          }}
        }} catch (_) {{}}
        return {{
          inFlightRequests: (diag.inFlightRequests || 0) + (diag.htmxInFlight || 0),
          requests: tracked.slice(-{})
        }};
      }})())
      "#,
      max
    );
    match eval_json_with_timeout(&wv, &script, 3000).await {
      Ok(val) => Ok(val),
      Err(_) => Ok(serde_json::json!({ "requests": [], "inFlightRequests": 0 })),
    }
  }

  pub async fn screenshot(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label, tab_url, tab_title, tab_err) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (
        mgr.app_handle.clone(),
        tab.label.clone(),
        tab.url.clone(),
        tab.title.clone(),
        tab.error_code.clone(),
      )
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;

    // Try native WebKitGTK snapshot on Linux first when the webview is visible
    #[cfg(target_os = "linux")]
    {
      if let Some(native_shot) = capture_linux_webkit_screenshot(&wv).await {
        return Ok(native_shot);
      }
    }

    // Cross-platform DOM-to-Canvas visual rasterizer (paints real computed styles, boxes, borders, inputs, tables, and text rects)
    let script = r#"
      JSON.stringify((function() {
        try {
          const vw = Math.max(320, Math.min(2560, window.innerWidth || document.documentElement?.clientWidth || 1280));
          const vh = Math.max(240, Math.min(2000, window.innerHeight || document.documentElement?.clientHeight || 800));
          const canvas = document.createElement('canvas');
          canvas.width = vw;
          canvas.height = vh;
          const ctx = canvas.getContext('2d');

          const isTransparent = (c) => !c || c === 'transparent' || c === 'rgba(0, 0, 0, 0)';

          const htmlBg = document.documentElement ? getComputedStyle(document.documentElement).backgroundColor : '';
          const bodyBg = document.body ? getComputedStyle(document.body).backgroundColor : '';
          ctx.fillStyle = !isTransparent(bodyBg) ? bodyBg : (!isTransparent(htmlBg) ? htmlBg : '#ffffff');
          ctx.fillRect(0, 0, vw, vh);

          const visible = (el) => {
            if (!el || el.nodeType !== 1) return false;
            if (el.id && el.id.startsWith('__open_browser')) return false;
            const style = getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity) === 0) return false;
            const r = el.getBoundingClientRect();
            return r.width > 0 && r.height > 0 && r.bottom > 0 && r.right > 0 && r.top < vh && r.left < vw;
          };

          const roundRectPath = (x, y, w, h, radius) => {
            const r = Math.max(0, Math.min(radius || 0, Math.min(w, h) / 2));
            ctx.beginPath();
            if (r > 0 && typeof ctx.roundRect === 'function') {
              ctx.roundRect(x, y, w, h, r);
            } else {
              ctx.rect(x, y, w, h);
            }
          };

          const allElements = Array.from(document.querySelectorAll('*')).filter(visible);

          for (const el of allElements) {
            const rect = el.getBoundingClientRect();
            const style = getComputedStyle(el);
            const radius = parseFloat(style.borderTopLeftRadius) || 0;

            // Paint background color
            if (!isTransparent(style.backgroundColor)) {
              ctx.save();
              roundRectPath(rect.left, rect.top, rect.width, rect.height, radius);
              ctx.fillStyle = style.backgroundColor;
              ctx.fill();
              ctx.restore();
            }

            // Paint borders
            const bwTop = parseFloat(style.borderTopWidth) || 0;
            const bwRight = parseFloat(style.borderRightWidth) || 0;
            const bwBottom = parseFloat(style.borderBottomWidth) || 0;
            const bwLeft = parseFloat(style.borderLeftWidth) || 0;
            if ((bwTop > 0 || bwRight > 0 || bwBottom > 0 || bwLeft > 0) && style.borderTopStyle !== 'none') {
              const bc = style.borderTopColor || style.borderColor;
              if (!isTransparent(bc)) {
                ctx.save();
                roundRectPath(rect.left, rect.top, rect.width, rect.height, radius);
                ctx.strokeStyle = bc;
                ctx.lineWidth = Math.max(1, Math.min(6, Math.max(bwTop, bwRight, bwBottom, bwLeft)));
                ctx.stroke();
                ctx.restore();
              }
            }

            // Paint images safely via untainted check
            const tag = el.tagName.toLowerCase();
            if (tag === 'img' && el.complete && el.naturalWidth > 0) {
              try {
                const testCanvas = document.createElement('canvas');
                testCanvas.width = 1;
                testCanvas.height = 1;
                const tctx = testCanvas.getContext('2d');
                tctx.drawImage(el, 0, 0, 1, 1);
                testCanvas.toDataURL();
                ctx.drawImage(el, rect.left, rect.top, rect.width, rect.height);
              } catch (_) {
                ctx.save();
                ctx.strokeStyle = '#8892b0';
                ctx.strokeRect(rect.left, rect.top, rect.width, rect.height);
                ctx.restore();
              }
            }

            // Paint input / textarea / select values
            if (tag === 'input' || tag === 'textarea' || tag === 'select') {
              const type = (el.getAttribute('type') || 'text').toLowerCase();
              if (type === 'checkbox' || type === 'radio') {
                ctx.save();
                ctx.strokeStyle = '#64748b';
                ctx.lineWidth = 1.5;
                ctx.strokeRect(rect.left + 2, rect.top + 2, rect.width - 4, rect.height - 4);
                if (el.checked) {
                  ctx.fillStyle = '#0284c7';
                  ctx.fillRect(rect.left + 5, rect.top + 5, Math.max(2, rect.width - 10), Math.max(2, rect.height - 10));
                }
                ctx.restore();
              } else if (type !== 'hidden') {
                let displayVal = '';
                if (tag === 'select') {
                  displayVal = el.options && el.selectedIndex >= 0 ? (el.options[el.selectedIndex]?.text || el.value) : el.value;
                } else if (type === 'password') {
                  displayVal = '•'.repeat((el.value || '').length);
                } else {
                  displayVal = el.value || el.getAttribute('placeholder') || '';
                }
                if (displayVal) {
                  ctx.save();
                  ctx.beginPath();
                  ctx.rect(rect.left + 4, rect.top, Math.max(0, rect.width - 8), rect.height);
                  ctx.clip();
                  ctx.font = `${style.fontWeight || '400'} ${style.fontSize || '14px'} ${style.fontFamily || 'sans-serif'}`;
                  ctx.fillStyle = el.value ? (style.color || '#111827') : '#9ca3af';
                  ctx.textBaseline = 'middle';
                  ctx.fillText(String(displayVal).slice(0, 120), rect.left + 8, rect.top + rect.height / 2);
                  ctx.restore();
                }
              }
            }
          }

          // Paint visible text nodes at their exact layout Range rects
          const rootNode = document.body || document.documentElement;
          if (rootNode) {
            const walker = document.createTreeWalker(rootNode, NodeFilter.SHOW_TEXT, null, false);
            let textNode;
            const range = document.createRange();
            while ((textNode = walker.nextNode())) {
              const raw = textNode.nodeValue;
              if (!raw || !raw.trim()) continue;
              const parent = textNode.parentElement;
              if (!parent || !visible(parent)) continue;
              const pTag = parent.tagName.toLowerCase();
              if (pTag === 'script' || pTag === 'style' || pTag === 'noscript' || pTag === 'select' || pTag === 'textarea') continue;

              const pStyle = getComputedStyle(parent);
              ctx.save();
              ctx.font = `${pStyle.fontStyle || 'normal'} ${pStyle.fontWeight || '400'} ${pStyle.fontSize || '14px'} ${pStyle.fontFamily || 'sans-serif'}`;
              ctx.fillStyle = pStyle.color || '#111827';
              ctx.textBaseline = 'top';

              try {
                range.selectNodeContents(textNode);
                const rects = range.getClientRects();
                const clean = raw.replace(/\s+/g, ' ').trim();
                if (rects.length === 1) {
                  const r = rects[0];
                  if (r.width > 0 && r.height > 0 && r.bottom > 0 && r.top < vh) {
                    ctx.fillText(clean, r.left, r.top + 1, r.width + 4);
                  }
                } else if (rects.length > 1) {
                  const pRect = parent.getBoundingClientRect();
                  const words = clean.split(' ');
                  let line = '';
                  let lineIdx = 0;
                  for (let i = 0; i < words.length && lineIdx < rects.length; i++) {
                    const test = line ? (line + ' ' + words[i]) : words[i];
                    const r = rects[lineIdx];
                    if (ctx.measureText(test).width > (r.width + 12) && line) {
                      if (r.bottom > 0 && r.top < vh) ctx.fillText(line, r.left, r.top + 1);
                      line = words[i];
                      lineIdx++;
                    } else {
                      line = test;
                    }
                  }
                  if (line && lineIdx < rects.length) {
                    const r = rects[lineIdx] || pRect;
                    if (r.bottom > 0 && r.top < vh) ctx.fillText(line, r.left, r.top + 1);
                  }
                }
              } catch (_) {}
              ctx.restore();
            }
          }

          const dataUrl = canvas.toDataURL('image/png');
          return {
            width: canvas.width,
            height: canvas.height,
            mimeType: 'image/png',
            dataUrl: dataUrl,
            base64: dataUrl.split(',')[1] || ''
          };
        } catch(e) {
          return { error: String(e) };
        }
      })())
    "#;

    match eval_json_with_timeout(&wv, script, 6000).await {
      Ok(val) if val.get("base64").and_then(|v| v.as_str()).map_or(false, |s| !s.is_empty()) => {
        Ok(val)
      }
      _ => {
        // Fallback diagnostic 1x1 valid PNG so screenshot never hangs or fails with an opaque timeout
        let fallback_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        Ok(serde_json::json!({
          "width": 1,
          "height": 1,
          "mimeType": "image/png",
          "dataUrl": format!("data:image/png;base64,{}", fallback_b64),
          "base64": fallback_b64,
          "url": tab_url,
          "title": tab_title,
          "errorCode": tab_err
        }))
      }
    }
  }
}

#[cfg(target_os = "linux")]
async fn capture_linux_webkit_screenshot(wv: &tauri::Webview) -> Option<serde_json::Value> {
  use base64::Engine;
  use gtk::prelude::WidgetExt;
  use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

  let (tx, rx) = tokio::sync::oneshot::channel();
  let tx_cell = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

  let _ = wv.with_webview(move |platform_wv| {
    let inner = platform_wv.inner();
    if !inner.is_visible() {
      if let Ok(mut lock) = tx_cell.lock() {
        if let Some(sender) = lock.take() {
          let _ = sender.send(None);
        }
      }
      return;
    }
    let alloc = inner.allocation();
    let width = alloc.width().max(1) as u32;
    let height = alloc.height().max(1) as u32;
    let tx_cb = tx_cell.clone();

    inner.snapshot(
      SnapshotRegion::Visible,
      SnapshotOptions::NONE,
      None::<&gtk::gio::Cancellable>,
      move |res| {
        let payload = match res {
          Ok(surface) => {
            if let Some(pixbuf) = gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, width as i32, height as i32) {
              if let Ok(png_bytes) = pixbuf.save_to_bufferv("png", &[]) {
                if !png_bytes.is_empty() {
                  let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
                  Some(serde_json::json!({
                    "width": width,
                    "height": height,
                    "mimeType": "image/png",
                    "dataUrl": format!("data:image/png;base64,{}", b64),
                    "base64": b64
                  }))
                } else {
                  None
                }
              } else {
                None
              }
            } else {
              None
            }
          }
          Err(_) => None,
        };
        if let Ok(mut lock) = tx_cb.lock() {
          if let Some(sender) = lock.take() {
            let _ = sender.send(payload);
          }
        }
      },
    );
  });

  match tokio::time::timeout(tokio::time::Duration::from_millis(2500), rx).await {
    Ok(Ok(Some(val))) => Some(val),
    _ => None,
  }
}

async fn wait_for_condition(
  wv: &tauri::Webview,
  timeout_ms: u64,
  until: Option<&str>,
  selector: Option<&str>,
) -> serde_json::Value {
  let _ = eval_js_with_timeout(wv, INIT_DIAGNOSTICS_SCRIPT, 600).await;

  if until.is_none() && selector.is_none() {
    tokio::time::sleep(tokio::time::Duration::from_millis(timeout_ms)).await;
    return serde_json::json!({ "waited": timeout_ms });
  }

  let start = tokio::time::Instant::now();
  let max_dur = tokio::time::Duration::from_millis(timeout_ms);
  let until_mode = until.unwrap_or("").to_lowercase();
  let sel_str = selector.unwrap_or("");

  // Initial short pause so newly triggered requests/mutations register
  tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

  let mut last_in_flight = 0;
  let mut last_mutations = 0;
  let mut matched = false;

  while start.elapsed() < max_dur {
    let check_script = format!(
      r#"
      JSON.stringify((function() {{
        const diag = window.__openBrowserDiag || {{}};
        const now = Date.now();
        const inFlight = (diag.inFlightRequests || 0) + (diag.htmxInFlight || 0);
        const idleForMs = now - (diag.lastNetworkActivityAt || 0);
        const domQuietForMs = now - (diag.lastMutationAt || 0);
        const ready = document.readyState === 'complete';
        const sel = {:?};
        let selectorMatched = true;
        if (sel) {{
          try {{
            const el = document.querySelector(sel);
            if (!el) {{
              selectorMatched = false;
            }} else {{
              const r = el.getBoundingClientRect();
              const s = getComputedStyle(el);
              selectorMatched = r.width > 0 && r.height > 0 && s.display !== 'none' && s.visibility !== 'hidden';
            }}
          }} catch (_) {{
            selectorMatched = false;
          }}
        }}
        return {{
          inFlight,
          idleForMs,
          domQuietForMs,
          ready,
          selectorMatched,
          mutationCount: diag.mutationCount || 0
        }};
      }})())
      "#,
      sel_str
    );

    if let Ok(state) = eval_json_with_timeout(wv, &check_script, 1000).await {
      last_in_flight = state.get("inFlight").and_then(|v| v.as_u64()).unwrap_or(0);
      last_mutations = state.get("mutationCount").and_then(|v| v.as_u64()).unwrap_or(0);
      let idle_for = state.get("idleForMs").and_then(|v| v.as_u64()).unwrap_or(0);
      let dom_quiet_for = state.get("domQuietForMs").and_then(|v| v.as_u64()).unwrap_or(0);
      let ready = state.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
      let sel_ok = state.get("selectorMatched").and_then(|v| v.as_bool()).unwrap_or(false);

      let until_ok = match until_mode.as_str() {
        "networkidle" => last_in_flight == 0 && idle_for >= 280 && dom_quiet_for >= 150 && ready,
        "domstable" => last_in_flight == 0 && dom_quiet_for >= 250,
        "load" => ready,
        "" => true,
        _ => last_in_flight == 0 && idle_for >= 250,
      };

      if sel_ok && until_ok {
        matched = true;
        break;
      }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
  }

  let elapsed_ms = start.elapsed().as_millis() as u64;
  serde_json::json!({
    "waited": elapsed_ms,
    "until": until,
    "selector": selector,
    "matched": matched,
    "timedOut": !matched,
    "inFlightRequests": last_in_flight,
    "domMutations": last_mutations
  })
}

async fn settle_after_interaction(wv: &tauri::Webview) {
  // Brief initial pause so HTMX/fetch/XHR, DOM swap, or link navigation triggered by click/press starts
  tokio::time::sleep(tokio::time::Duration::from_millis(65)).await;

  let start = tokio::time::Instant::now();
  let max_settle = tokio::time::Duration::from_millis(3000);

  let script = r#"
    JSON.stringify((function() {
      const diag = window.__openBrowserDiag || {};
      const now = Date.now();
      const inFlight = (diag.inFlightRequests || 0) + (diag.htmxInFlight || 0);
      const netAge = now - (diag.lastNetworkActivityAt || 0);
      const mutAge = now - (diag.lastMutationAt || 0);
      const loading = document.readyState !== 'complete';
      const unloading = Boolean(window.__openBrowserUnloading) && (now - window.__openBrowserUnloading < 2500);
      return {
        busy: unloading || inFlight > 0 || loading || netAge < 140 || mutAge < 120
      };
    })())
  "#;

  while start.elapsed() < max_settle {
    match eval_json_with_timeout(wv, script, 600).await {
      Ok(val) => {
        let busy = val.get("busy").and_then(|v| v.as_bool()).unwrap_or(false);
        if !busy {
          break;
        }
      }
      Err(_) => break,
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(55)).await;
  }
}

async fn eval_js_async(wv: &tauri::Webview, script: &str) -> Result<String, String> {
  eval_js_with_timeout(wv, script, 10000).await
}

async fn eval_js_with_timeout(
  wv: &tauri::Webview,
  script: &str,
  timeout_ms: u64,
) -> Result<String, String> {
  let (tx, rx) = tokio::sync::oneshot::channel();
  let tx_cell = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
  wv.eval_with_callback(script, move |result| {
    if let Ok(mut lock) = tx_cell.lock() {
      if let Some(sender) = lock.take() {
        let _ = sender.send(result);
      }
    }
  })
  .map_err(|e| format!("Error enviando eval: {}", e))?;

  match tokio::time::timeout(tokio::time::Duration::from_millis(timeout_ms), rx).await {
    Ok(Ok(val)) => Ok(val),
    Ok(Err(_)) => Err("El canal de respuesta de JavaScript se cerró".into()),
    Err(_) => Err("Timeout esperando respuesta de la página web".into()),
  }
}

async fn eval_json_async(wv: &tauri::Webview, script: &str) -> Result<serde_json::Value, String> {
  eval_json_with_timeout(wv, script, 10000).await
}

async fn eval_json_with_timeout(
  wv: &tauri::Webview,
  script: &str,
  timeout_ms: u64,
) -> Result<serde_json::Value, String> {
  let raw = eval_js_with_timeout(wv, script, timeout_ms).await?;
  match serde_json::from_str::<serde_json::Value>(&raw) {
    Ok(serde_json::Value::String(inner)) => serde_json::from_str::<serde_json::Value>(&inner)
      .or_else(|_| Ok(serde_json::Value::String(inner))),
    Ok(val) => Ok(val),
    Err(_) => Ok(serde_json::json!({ "raw": raw })),
  }
}

async fn execute_visual_action(
  wv: &tauri::Webview,
  action: &serde_json::Value,
  fast: bool,
) -> Result<serde_json::Value, String> {
  let script = crate::visual_agent::build_visual_action_script(action, fast);
  let _ = eval_js_async(wv, &script).await?;

  let check_script = r#"
    JSON.stringify(window.__open_browser_action_status || { done: true, ok: true })
  "#;

  let start = tokio::time::Instant::now();
  loop {
    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    if let Ok(val) = eval_json_with_timeout(wv, check_script, 1500).await {
      if val.get("done").and_then(|v| v.as_bool()) == Some(true) {
        if val.get("ok").and_then(|v| v.as_bool()) == Some(true) {
          return Ok(serde_json::json!({ "ok": true }));
        } else {
          let err_msg = val
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Error en acción");
          return Err(err_msg.to_string());
        }
      }
    }

    if start.elapsed() > tokio::time::Duration::from_secs(10) {
      return Err("Timeout esperando que la acción visual complete".into());
    }
  }
}
