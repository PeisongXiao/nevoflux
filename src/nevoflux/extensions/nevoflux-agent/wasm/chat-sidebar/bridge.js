/**
 * NevoFlux Bridge
 * Connects Dioxus WASM to browser.nevoflux Experiment API
 */

(function() {
  "use strict";

  // Event subscriptions
  const subscriptions = {
    selection: new Set(),
  };

  // Initialize event listeners
  function initEventListeners() {
    if (typeof browser !== "undefined" && browser.nevoflux?.onSelectionChanged) {
      browser.nevoflux.onSelectionChanged.addListener((tabId, selection) => {
        const event = JSON.stringify({ tabId, selection });
        subscriptions.selection.forEach(callback => {
          try {
            callback(event);
          } catch (e) {
            console.error("[NevofluxBridge] Selection callback error:", e);
          }
        });
      });
    }
  }

  // Bridge API exposed to WASM
  window.NevofluxBridge = {
    // Check if API is available
    isAvailable() {
      return typeof browser !== "undefined" && typeof browser.nevoflux !== "undefined";
    },

    // ==================== Tab Content ====================

    async getTabContent(tabId, optionsJson) {
      try {
        const options = optionsJson ? JSON.parse(optionsJson) : {};
        const result = await browser.nevoflux.getTabContent(tabId, options);
        return JSON.stringify({ success: true, data: result });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    async getTabState(tabId) {
      try {
        const result = await browser.nevoflux.getTabState(tabId);
        return JSON.stringify({ success: true, data: result });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    async getAllTabs() {
      try {
        const tabs = await browser.tabs.query({});
        const tabInfos = tabs.map(tab => ({
          id: tab.id,
          url: tab.url || "",
          title: tab.title || "",
          active: tab.active,
          discarded: tab.discarded || false,
          favIconUrl: tab.favIconUrl || null,
        }));
        return JSON.stringify({ success: true, data: tabInfos });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    async getActiveTab() {
      try {
        const tabs = await browser.tabs.query({ active: true, currentWindow: true });
        if (tabs.length === 0) {
          return JSON.stringify({ success: false, error: "No active tab" });
        }
        const tab = tabs[0];
        return JSON.stringify({
          success: true,
          data: {
            id: tab.id,
            url: tab.url || "",
            title: tab.title || "",
            active: true,
            discarded: tab.discarded || false,
            favIconUrl: tab.favIconUrl || null,
          }
        });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    // ==================== Element Picker ====================

    async pickElement(tabId, optionsJson) {
      try {
        const options = optionsJson ? JSON.parse(optionsJson) : {};
        const result = await browser.nevoflux.pickElement(tabId, options);
        return JSON.stringify({ success: true, data: result });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    async cancelPicker(tabId) {
      try {
        await browser.nevoflux.cancelPicker(tabId);
        return JSON.stringify({ success: true });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    // ==================== Selection ====================

    async getSelection(tabId) {
      try {
        const result = await browser.nevoflux.getSelection(tabId);
        return JSON.stringify({ success: true, data: result });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    subscribeSelection(callback) {
      subscriptions.selection.add(callback);
      // Return unsubscribe function ID (for cleanup)
      const id = Date.now().toString();
      return id;
    },

    unsubscribeSelection(callback) {
      subscriptions.selection.delete(callback);
    },

    // ==================== Page Lock ====================

    async lockPage(tabId, optionsJson) {
      try {
        const options = optionsJson ? JSON.parse(optionsJson) : {};
        await browser.nevoflux.lockPage(tabId, options);
        return JSON.stringify({ success: true });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },

    async unlockPage(tabId) {
      try {
        await browser.nevoflux.unlockPage(tabId);
        return JSON.stringify({ success: true });
      } catch (e) {
        return JSON.stringify({ success: false, error: e.message });
      }
    },
  };

  // Initialize
  initEventListeners();
  console.log("[NevofluxBridge] Initialized, API available:", window.NevofluxBridge.isAvailable());
})();
