/**
 * dom-watcher.js — content script that publishes DOM mutation events
 * to the daemon's EventBus for the /loop skill's state:tab=…:<sel>:change
 * trigger (spec §5.2 / §9.1).
 *
 * MVP scope: one MutationObserver on document.body per page; debounced
 * to 300ms. Each batch publishes `ui:tab:dom:mutation` (no selector
 * specificity in MVP — daemon-side filtering / LLM-side refinement via
 * browser_query handles that).
 */

(() => {
  if (window.__nevoflux_dom_watcher_installed__) return;
  window.__nevoflux_dom_watcher_installed__ = true;

  let timer = null;
  const DEBOUNCE_MS = 300;

  function publishMutation() {
    timer = null;
    try {
      browser.runtime.sendMessage({
        type: 'bg:events_publish',
        topic: 'ui:tab:dom:mutation',
        data: {
          url: location.href,
          ts_ms: Date.now(),
        },
        delivery: 'ephemeral',
      }).catch((e) => {
        console.debug('[nevoflux dom-watcher] publish failed:', e);
      });
    } catch (e) {
      console.debug('[nevoflux dom-watcher] publish exception:', e);
    }
  }

  function onMutation() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(publishMutation, DEBOUNCE_MS);
  }

  function attach() {
    if (!document.body) {
      requestAnimationFrame(attach);
      return;
    }
    const observer = new MutationObserver(onMutation);
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: false,
      characterData: false,
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', attach, { once: true });
  } else {
    attach();
  }
})();
