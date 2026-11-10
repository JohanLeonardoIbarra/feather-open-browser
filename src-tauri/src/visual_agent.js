window.__open_browser_action_status = { done: false, ok: false, error: null };
(async () => {
  try {
    const action = __ACTION_JSON__;
    const fast = __FAST_BOOL__;

    const STYLE_ID = '__open_browser_visual_styles';
    if (!document.getElementById(STYLE_ID)) {
      const style = document.createElement('style');
      style.id = STYLE_ID;
      style.textContent = `
        #__open_browser_cursor {
          position: fixed;
          top: 0;
          left: 0;
          width: 28px;
          height: 28px;
          pointer-events: none !important;
          z-index: 2147483647 !important;
          opacity: 0;
          transform: translate(-100px, -100px);
          will-change: transform, opacity;
          transition: opacity 0.35s ease;
        }
        #__open_browser_cursor.ob-visible {
          opacity: 1 !important;
        }
        .ob-cursor-inner {
          position: relative;
          width: 100%;
          height: 100%;
          transform-origin: 3px 2px;
          will-change: transform;
        }
        .ob-cursor-svg {
          width: 24px;
          height: 24px;
          filter: drop-shadow(0 2px 7px rgba(0, 0, 0, 0.65)) drop-shadow(0 0 10px rgba(0, 240, 255, 0.45));
        }
        .ob-cursor-badge {
          position: absolute;
          left: 22px;
          top: 13px;
          background: rgba(13, 17, 23, 0.94);
          backdrop-filter: blur(8px);
          border: 1px solid #00f0ff;
          color: #e6edf3;
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
          font-size: 11px;
          font-weight: 600;
          padding: 3px 9px;
          border-radius: 6px;
          white-space: nowrap;
          box-shadow: 0 4px 14px rgba(0, 0, 0, 0.5), 0 0 10px rgba(0, 240, 255, 0.25);
          pointer-events: none !important;
          line-height: 1.3;
          opacity: 0;
          transform: translateY(4px);
          transition: opacity 0.16s ease, transform 0.16s cubic-bezier(0.18, 0.89, 0.32, 1.2);
          display: none;
        }
        .ob-cursor-badge.ob-badge-show {
          display: block !important;
          opacity: 1 !important;
          transform: translateY(0) !important;
        }
        .ob-click-ripple {
          position: fixed;
          width: 16px;
          height: 16px;
          border-radius: 50%;
          pointer-events: none !important;
          z-index: 2147483646 !important;
          border: 2px solid #00f0ff;
          background: radial-gradient(circle, rgba(0, 240, 255, 0.35) 0%, rgba(0, 240, 255, 0) 70%);
          box-shadow: 0 0 12px #00f0ff, inset 0 0 8px rgba(0, 240, 255, 0.5);
          transform: translate(-50%, -50%) scale(0.2);
          opacity: 1;
          transition: transform 0.38s cubic-bezier(0.1, 0.82, 0.25, 1), opacity 0.38s ease-out;
          will-change: transform, opacity;
        }
        .ob-click-ripple.ob-ripple-expand {
          transform: translate(-50%, -50%) scale(3.2) !important;
          opacity: 0 !important;
        }
        .ob-cursor-trail {
          position: fixed;
          width: 7px;
          height: 7px;
          border-radius: 50%;
          pointer-events: none !important;
          z-index: 2147483645 !important;
          background: #00f0ff;
          box-shadow: 0 0 9px rgba(0, 240, 255, 0.85);
          transform: translate(-50%, -50%) scale(1);
          opacity: 0.7;
          transition: transform 0.3s ease-out, opacity 0.3s ease-out;
          will-change: transform, opacity;
        }
        .ob-cursor-trail.ob-trail-fade {
          transform: translate(-50%, -50%) scale(0.15) !important;
          opacity: 0 !important;
        }
        .ob-element-highlight {
          outline: 2px solid #00f0ff !important;
          outline-offset: 3px !important;
          box-shadow: 0 0 14px rgba(0, 240, 255, 0.65) !important;
          transition: outline 0.15s ease, box-shadow 0.15s ease !important;
        }
      `;
      (document.head || document.documentElement).appendChild(style);
    }

    let cursor = document.getElementById('__open_browser_cursor');
    let inner = null;
    let badge = null;

    if (!cursor) {
      cursor = document.createElement('div');
      cursor.id = '__open_browser_cursor';
      cursor.innerHTML = `
        <div class="ob-cursor-inner">
          <svg class="ob-cursor-svg" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M3 2L3 18.5L7.2 14.3L11.5 22L13.8 20.7L9.5 13.2L15.5 13.2L3 2Z" fill="#00f0ff" stroke="#06121e" stroke-width="1.8" stroke-linejoin="round"/>
          </svg>
          <div id="__open_browser_cursor_badge" class="ob-cursor-badge"></div>
        </div>
      `;
      (document.body || document.documentElement).appendChild(cursor);
    }
    inner = cursor.querySelector('.ob-cursor-inner');
    badge = cursor.querySelector('#__open_browser_cursor_badge');

    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

    const state = window.__openBrowserCursorState = window.__openBrowserCursorState || {
      x: null,
      y: null,
      fadeTimer: null
    };

    const triggerRipple = (x, y) => {
      if (fast) return;
      const ripple = document.createElement('div');
      ripple.className = 'ob-click-ripple';
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';
      (document.body || document.documentElement).appendChild(ripple);
      requestAnimationFrame(() => {
        ripple.classList.add('ob-ripple-expand');
      });
      setTimeout(() => ripple.remove(), 420);
    };

    const spawnTrail = (x, y) => {
      if (fast) return;
      const trail = document.createElement('div');
      trail.className = 'ob-cursor-trail';
      trail.style.left = x + 'px';
      trail.style.top = y + 'px';
      (document.body || document.documentElement).appendChild(trail);
      requestAnimationFrame(() => {
        trail.classList.add('ob-trail-fade');
      });
      setTimeout(() => trail.remove(), 320);
    };

    const showBadge = (text) => {
      if (fast || !badge) return;
      badge.textContent = text;
      badge.style.display = 'block';
      requestAnimationFrame(() => {
        badge.classList.add('ob-badge-show');
      });
    };

    const hideBadge = () => {
      if (!badge) return;
      badge.classList.remove('ob-badge-show');
      setTimeout(() => {
        if (!badge.classList.contains('ob-badge-show')) {
          badge.style.display = 'none';
        }
      }, 160);
    };

    // Smooth Bezier curved flight with banking tilt and particle trail
    const moveCursorTo = async (targetX, targetY, badgeText) => {
      if (fast) return;

      clearTimeout(state.fadeTimer);

      const hasPrevPos = Number.isFinite(state.x) && Number.isFinite(state.y);
      let startX = hasPrevPos ? state.x : Math.max(30, Math.min(window.innerWidth - 30, window.innerWidth * 0.18));
      let startY = hasPrevPos ? state.y : Math.max(30, Math.min(window.innerHeight - 30, window.innerHeight * 0.78));

      const dx = targetX - startX;
      const dy = targetY - startY;
      const dist = Math.hypot(dx, dy);

      cursor.classList.add('ob-visible');
      if (!hasPrevPos) {
        cursor.style.transform = `translate(${startX}px, ${startY}px)`;
        cursor.style.opacity = '0';
      }

      showBadge(badgeText);

      if (dist > 4) {
        const midX = (startX + targetX) / 2;
        const midY = (startY + targetY) / 2;
        const nx = -dy / dist;
        const ny = dx / dist;
        const arcSign = ((dx * dy) >= 0) ? -1 : 1;
        const arcMag = Math.min(85, Math.max(16, dist * 0.18)) * arcSign;
        const ctrlX = midX + nx * arcMag;
        const ctrlY = midY + ny * arcMag;

        const duration = Math.min(520, Math.max(220, Math.round(160 + Math.sqrt(dist) * 14)));
        const startTime = performance.now();
        let lastTrailTime = 0;

        await new Promise((resolve) => {
          const stepFrame = (now) => {
            const elapsed = now - startTime;
            const progress = Math.min(1, elapsed / duration);

            const ease = progress < 0.5
              ? 4 * progress * progress * progress
              : 1 - Math.pow(-2 * progress + 2, 3) / 2;

            const inv = 1 - ease;
            const curX = inv * inv * startX + 2 * inv * ease * ctrlX + ease * ease * targetX;
            const curY = inv * inv * startY + 2 * inv * ease * ctrlY + ease * ease * targetY;

            if (!hasPrevPos && progress < 0.25) {
              cursor.style.opacity = String(progress / 0.25);
            } else {
              cursor.style.opacity = '1';
            }

            const tx = 2 * inv * (ctrlX - startX) + 2 * ease * (targetX - ctrlX);
            const ty = 2 * inv * (ctrlY - startY) + 2 * ease * (targetY - ctrlY);
            const tMag = Math.hypot(tx, ty) || 1;
            const bankAngle = (tx / tMag) * 14 * (1 - ease);

            cursor.style.transform = `translate(${curX.toFixed(1)}px, ${curY.toFixed(1)}px)`;
            if (inner) {
              inner.style.transform = `rotate(${bankAngle.toFixed(1)}deg)`;
            }

            if (dist > 50 && progress < 0.94 && (now - lastTrailTime) > 34) {
              lastTrailTime = now;
              spawnTrail(curX, curY);
            }

            if (progress < 1) {
              requestAnimationFrame(stepFrame);
            } else {
              resolve();
            }
          };
          requestAnimationFrame(stepFrame);
        });
      }

      cursor.style.transform = `translate(${targetX}px, ${targetY}px)`;
      cursor.style.opacity = '1';
      if (inner) inner.style.transform = 'rotate(0deg) scale(1)';
      state.x = targetX;
      state.y = targetY;
      await wait(50);
    };

    const scheduleFadeout = () => {
      clearTimeout(state.fadeTimer);
      state.fadeTimer = setTimeout(() => {
        if (cursor) {
          cursor.style.opacity = '0';
          setTimeout(() => {
            cursor.classList.remove('ob-visible');
          }, 360);
        }
      }, 30000);
    };

    const getElementLabel = (el) => {
      const text = el.getAttribute('aria-label') || el.innerText || el.value || el.placeholder || el.name || el.id || '';
      const clean = text.replace(/\s+/g, ' ').trim();
      return clean.length > 18 ? clean.slice(0, 16) + '…' : clean;
    };

    // Execute action with full visual flair
    if (action.type === 'click') {
      const el = window.__openBrowserAgentRefs?.get(action.ref);
      if (!el?.isConnected) throw new Error('Elemento no encontrado o desconectado; toma un snapshot nuevo.');

      const rect = el.getBoundingClientRect();
      const isOffscreen = rect.top < 0 || rect.bottom > window.innerHeight || rect.left < 0 || rect.right > window.innerWidth;
      if (isOffscreen) {
        el.scrollIntoView({ behavior: fast ? 'auto' : 'smooth', block: 'center', inline: 'nearest' });
        if (!fast) await wait(120);
      }

      // If el contains a specific text element (like .chapter-item-headtitle, title, or inner text container),
      // target the text itself directly so the cursor clicks precisely on the words.
      let targetForClick = el;
      let targetRect = el.getBoundingClientRect();

      const textChild = el.querySelector('.chapter-item-headtitle, [class*="headtitle"], [class*="title"], [class*="name"], [class*="label"], span, strong, b, h1, h2, h3, h4, h5, h6');
      if (textChild) {
        const tr = textChild.getBoundingClientRect();
        if (tr.width > 0 && tr.height > 0) {
          targetForClick = textChild;
          targetRect = tr;
        }
      } else {
        try {
          const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null, false);
          let node;
          while ((node = walker.nextNode())) {
            if (node.textContent && node.textContent.trim().length > 0) {
              const range = document.createRange();
              range.selectNodeContents(node);
              const rr = range.getBoundingClientRect();
              if (rr.width > 0 && rr.height > 0) {
                targetRect = rr;
                break;
              }
            }
          }
        } catch (_) {}
      }

      const x = Math.max(4, Math.min(window.innerWidth - 8, targetRect.left + targetRect.width / 2));
      const y = Math.max(4, Math.min(window.innerHeight - 8, targetRect.top + targetRect.height / 2));
      const label = getElementLabel(targetForClick) || getElementLabel(el) || action.ref;

      await moveCursorTo(x, y, `🖱️ Clic · ${label}`);

      if (!fast && inner) {
        inner.style.transition = 'transform 0.07s cubic-bezier(0.4, 0, 0.2, 1)';
        inner.style.transform = 'scale(0.82) translate(1px, 2px) rotate(-4deg)';
        await wait(70);
      }

      try { (targetForClick || el).classList.add('ob-element-highlight'); } catch {}
      triggerRipple(x, y);

      const anchor = el.tagName === 'A' ? el : el.closest('a');
      if (anchor && anchor.target && anchor.target.toLowerCase() === '_blank') {
        anchor.target = '_self';
      }

      try {
        targetForClick.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, view: window, clientX: x, clientY: y }));
        targetForClick.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, view: window, clientX: x, clientY: y }));
        targetForClick.click();
      } catch (_) {}

      if (targetForClick !== el) {
        try { el.click(); } catch {}
      }

      if (!fast && inner) {
        inner.style.transition = 'transform 0.16s cubic-bezier(0.175, 0.885, 0.32, 1.35)';
        inner.style.transform = 'scale(1.08) rotate(0deg)';
        await wait(140);
        inner.style.transition = 'transform 0.08s ease-out';
        inner.style.transform = 'scale(1) rotate(0deg)';
        await wait(50);
        inner.style.transition = '';
      }

      try { el.classList.remove('ob-element-highlight'); } catch {}
      hideBadge();
      scheduleFadeout();

      window.__open_browser_action_status = { done: true, ok: true };
      return;
    }

    if (action.type === 'fill') {
      const el = window.__openBrowserAgentRefs?.get(action.ref);
      if (!el?.isConnected) throw new Error('Elemento no encontrado o desconectado; toma un snapshot nuevo.');

      const valStr = String(action.value ?? action.text ?? '');
      const rect = el.getBoundingClientRect();
      const isOffscreen = rect.top < 0 || rect.bottom > window.innerHeight || rect.left < 0 || rect.right > window.innerWidth;
      if (isOffscreen) {
        el.scrollIntoView({ behavior: fast ? 'auto' : 'smooth', block: 'center', inline: 'nearest' });
        if (!fast) await wait(120);
      }

      const freshRect = el.getBoundingClientRect();
      const x = Math.max(4, Math.min(window.innerWidth - 8, freshRect.left + 12));
      const y = Math.max(4, Math.min(window.innerHeight - 8, freshRect.top + freshRect.height / 2));
      const preview = valStr.length > 16 ? valStr.slice(0, 14) + '…' : valStr;

      await moveCursorTo(x, y, `⌨️ Escribir · "${preview}"`);

      try { el.classList.add('ob-element-highlight'); } catch {}
      el.focus();

      // Type visibly into the element
      if (valStr.length > 0 && !fast && valStr.length <= 40) {
        let currentVal = '';
        for (let i = 0; i < valStr.length; i++) {
          currentVal += valStr[i];
          if (el.isContentEditable) {
            el.textContent = currentVal;
          } else if ('value' in el) {
            const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), 'value')?.set;
            if (setter) setter.call(el, currentVal);
            else el.value = currentVal;
          }
          el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: valStr[i] }));
          await wait(Math.min(28, Math.max(12, 280 / valStr.length)));
        }
      } else {
        if (el.isContentEditable) {
          el.textContent = valStr;
        } else if ('value' in el) {
          const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), 'value')?.set;
          if (setter) setter.call(el, valStr);
          else el.value = valStr;
        } else {
          throw new Error('El elemento no es editable.');
        }
        el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: valStr }));
      }

      el.dispatchEvent(new Event('change', { bubbles: true }));

      if (!fast) await wait(120);
      try { el.classList.remove('ob-element-highlight'); } catch {}
      hideBadge();
      scheduleFadeout();

      window.__open_browser_action_status = { done: true, ok: true };
      return;
    }

    if (action.type === 'press') {
      const el = (action.ref ? window.__openBrowserAgentRefs?.get(action.ref) : null) || document.activeElement || document.body;
      if (!el) throw new Error('No hay elemento activo para presionar la tecla.');

      if (!fast) {
        const rect = el.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          const x = Math.max(4, Math.min(window.innerWidth - 8, rect.left + rect.width / 2));
          const y = Math.max(4, Math.min(window.innerHeight - 8, rect.top + rect.height / 2));
          await moveCursorTo(x, y, `⤶ Tecla [${action.key || 'Enter'}]`);
        }
      }

      const key = action.key === 'Space' ? ' ' : (action.key || 'Enter');
      const init = {
        key,
        code: key === 'Enter' ? 'Enter' : key,
        keyCode: key === 'Enter' ? 13 : 0,
        which: key === 'Enter' ? 13 : 0,
        bubbles: true,
        cancelable: true
      };
      el.dispatchEvent(new KeyboardEvent('keydown', init));
      el.dispatchEvent(new KeyboardEvent('keypress', init));
      const form = el.form || el.closest('form');
      if ((key === 'Enter' || action.key === 'Enter') && form && !(el instanceof HTMLTextAreaElement)) {
        try { form.requestSubmit(); } catch { form.submit(); }
      }
      el.dispatchEvent(new KeyboardEvent('keyup', init));

      if (!fast) await wait(100);
      hideBadge();
      scheduleFadeout();

      window.__open_browser_action_status = { done: true, ok: true };
      return;
    }

    if (action.type === 'hover') {
      const el = window.__openBrowserAgentRefs?.get(action.ref);
      if (!el?.isConnected) throw new Error('Elemento no encontrado para hover.');

      const rect = el.getBoundingClientRect();
      const isOffscreen = rect.top < 0 || rect.bottom > window.innerHeight || rect.left < 0 || rect.right > window.innerWidth;
      if (isOffscreen) {
        el.scrollIntoView({ behavior: fast ? 'auto' : 'smooth', block: 'center', inline: 'nearest' });
        if (!fast) await wait(120);
      }

      const freshRect = el.getBoundingClientRect();
      const x = Math.max(4, Math.min(window.innerWidth - 8, freshRect.left + freshRect.width / 2));
      const y = Math.max(4, Math.min(window.innerHeight - 8, freshRect.top + freshRect.height / 2));
      const label = getElementLabel(el) || action.ref;

      await moveCursorTo(x, y, `👁️ Inspeccionar · ${label}`);
      try { el.classList.add('ob-element-highlight'); } catch {}

      el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
      el.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));

      if (!fast) await wait(200);
      try { el.classList.remove('ob-element-highlight'); } catch {}
      hideBadge();
      scheduleFadeout();

      window.__open_browser_action_status = { done: true, ok: true };
      return;
    }

    if (action.type === 'scroll') {
      const dx = Number(action.deltaX || 0);
      const dy = Number(action.deltaY || 0);

      if (!fast) {
        const dir = dy > 0 ? '↓' : (dy < 0 ? '↑' : (dx > 0 ? '→' : '←'));
        const cx = window.innerWidth / 2;
        const cy = window.innerHeight / 2;
        await moveCursorTo(cx, cy, `↕️ Desplazar [${dir}]`);
      }

      window.scrollBy({ top: dy, left: dx, behavior: fast ? 'auto' : 'smooth' });

      if (!fast) await wait(220);
      hideBadge();
      scheduleFadeout();

      window.__open_browser_action_status = { done: true, ok: true };
      return;
    }

    window.__open_browser_action_status = { done: true, ok: false, error: 'Acción no soportada en overlay' };
  } catch (err) {
    window.__open_browser_action_status = { done: true, ok: false, error: String(err?.message || err) };
  }
})();
