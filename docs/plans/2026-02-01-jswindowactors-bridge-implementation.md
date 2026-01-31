# JSWindowActors + WebExtension Bridge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a bridge between JSWindowActors and WebExtension to enable the Dioxus sidebar to access high-performance page interaction capabilities (element picker, selection sync, tab content extraction with auto-restore).

**Architecture:** Hybrid approach using existing JSWindowActors (`NevofluxChild`/`NevofluxParent`) with a new WebExtension Experiment API bridge. The sidebar calls `browser.nevoflux.*` which routes through the Experiment API to the Actors.

**Tech Stack:** Firefox JSWindowActors, WebExtension Experiment API, Dioxus/WASM, Rust wasm-bindgen

**Current State:**
- JSWindowActors already exist in `src/nevoflux/engine-overlays/browser/actors/`
- Actor registration exists in `DesktopActorRegistry.sys.mjs`
- `NevofluxChild` already has `getMarkdown()` and browser control actions
- No Experiment API exists yet
- No JS bridge for WASM exists yet

---

## Phase 1: Experiment API Foundation

### Task 1.1: Create API Schema

**Files:**
- Create: `src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/schema.json`

**Step 1: Create directory structure**

```bash
mkdir -p src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux
```

**Step 2: Create schema.json**

```json
[
  {
    "namespace": "nevoflux",
    "description": "NevoFlux Agent API for page interaction",
    "types": [
      {
        "id": "TabContent",
        "type": "object",
        "properties": {
          "tabId": { "type": "integer" },
          "url": { "type": "string" },
          "title": { "type": "string" },
          "content": { "type": "string" },
          "format": { "type": "string", "enum": ["markdown", "html", "text"] },
          "extractedAt": { "type": "integer" },
          "wasDiscarded": { "type": "boolean" }
        }
      },
      {
        "id": "TabState",
        "type": "object",
        "properties": {
          "discarded": { "type": "boolean" },
          "status": { "type": "string", "enum": ["complete", "loading", "unloaded"] },
          "url": { "type": "string" },
          "title": { "type": "string" }
        }
      },
      {
        "id": "PickerResult",
        "type": "object",
        "properties": {
          "selector": { "type": "string" },
          "xpath": { "type": "string" },
          "tagName": { "type": "string" },
          "id": { "type": "string", "optional": true },
          "className": { "type": "string", "optional": true },
          "text": { "type": "string", "optional": true },
          "attributes": { "type": "object" },
          "rect": { "type": "object" }
        }
      },
      {
        "id": "SelectionData",
        "type": "object",
        "properties": {
          "text": { "type": "string" },
          "html": { "type": "string" },
          "rect": { "type": "object" },
          "anchorNode": { "type": "string" },
          "url": { "type": "string" },
          "title": { "type": "string" }
        }
      }
    ],
    "functions": [
      {
        "name": "getTabContent",
        "type": "function",
        "async": true,
        "description": "Get page content as markdown/html/text. Auto-restores discarded tabs.",
        "parameters": [
          { "name": "tabId", "type": "integer" },
          {
            "name": "options",
            "type": "object",
            "optional": true,
            "properties": {
              "format": { "type": "string", "enum": ["markdown", "html", "text"], "optional": true },
              "selector": { "type": "string", "optional": true },
              "autoRestore": { "type": "boolean", "optional": true },
              "keepRestored": { "type": "boolean", "optional": true },
              "timeout": { "type": "integer", "optional": true }
            }
          }
        ]
      },
      {
        "name": "getTabState",
        "type": "function",
        "async": true,
        "description": "Get tab state (discarded, loading, complete)",
        "parameters": [
          { "name": "tabId", "type": "integer" }
        ]
      },
      {
        "name": "pickElement",
        "type": "function",
        "async": true,
        "description": "Start element picker and wait for user selection",
        "parameters": [
          { "name": "tabId", "type": "integer" },
          {
            "name": "options",
            "type": "object",
            "optional": true,
            "properties": {
              "hint": { "type": "string", "optional": true },
              "filter": { "type": "string", "enum": ["any", "button", "input", "link", "image", "clickable"], "optional": true },
              "timeout": { "type": "integer", "optional": true },
              "highlightColor": { "type": "string", "optional": true }
            }
          }
        ]
      },
      {
        "name": "cancelPicker",
        "type": "function",
        "async": true,
        "description": "Cancel active element picker",
        "parameters": [
          { "name": "tabId", "type": "integer" }
        ]
      },
      {
        "name": "getSelection",
        "type": "function",
        "async": true,
        "description": "Get current text selection from a tab",
        "parameters": [
          { "name": "tabId", "type": "integer" }
        ]
      },
      {
        "name": "lockPage",
        "type": "function",
        "async": true,
        "description": "Lock page to prevent user interaction",
        "parameters": [
          { "name": "tabId", "type": "integer" },
          {
            "name": "options",
            "type": "object",
            "optional": true,
            "properties": {
              "showOverlay": { "type": "boolean", "optional": true },
              "message": { "type": "string", "optional": true }
            }
          }
        ]
      },
      {
        "name": "unlockPage",
        "type": "function",
        "async": true,
        "description": "Unlock page after agent operations",
        "parameters": [
          { "name": "tabId", "type": "integer" }
        ]
      }
    ],
    "events": [
      {
        "name": "onSelectionChanged",
        "type": "function",
        "description": "Fired when text selection changes in any tab",
        "parameters": [
          { "name": "tabId", "type": "integer" },
          { "name": "selection", "$ref": "SelectionData", "optional": true }
        ]
      }
    ]
  }
]
```

**Step 3: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/experiment-apis/
git commit -m "feat(agent): add experiment API schema for nevoflux bridge"
```

---

### Task 1.2: Create Experiment API Implementation (Skeleton)

**Files:**
- Create: `src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs`

**Step 1: Create api.sys.mjs with basic structure**

```javascript
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

"use strict";

const { ExtensionCommon } = ChromeUtils.importESModule(
  "resource://gre/modules/ExtensionCommon.sys.mjs"
);

const { ExtensionError } = ExtensionCommon;

const lazy = {};

ChromeUtils.defineESModuleGetters(lazy, {
  SessionStore: "resource:///modules/sessionstore/SessionStore.sys.mjs",
});

/**
 * NevoFlux Experiment API
 * Bridges WebExtension to JSWindowActors for high-performance page interaction
 */
this.nevoflux = class extends ExtensionAPI {
  getAPI(context) {
    const { extension } = context;

    // Helper: Get native tab from tabId
    const getNativeTab = (tabId) => {
      const tab = extension.tabManager.get(tabId);
      if (!tab) {
        throw new ExtensionError(`Tab ${tabId} not found`);
      }
      return tab.nativeTab;
    };

    // Helper: Get browser from native tab
    const getBrowser = (nativeTab) => {
      return nativeTab.linkedBrowser;
    };

    // Helper: Get Actor for a tab
    const getActor = (nativeTab) => {
      const browser = getBrowser(nativeTab);
      const actor = browser?.browsingContext?.currentWindowGlobal?.getActor("Nevoflux");
      if (!actor) {
        throw new ExtensionError("Cannot get Nevoflux actor for this tab");
      }
      return actor;
    };

    // Helper: Check if tab is discarded
    const isTabDiscarded = (nativeTab) => {
      return nativeTab.hasAttribute("pending");
    };

    // Helper: Restore discarded tab (silent, no tab switch)
    const restoreTabIfNeeded = async (nativeTab, timeout = 30000) => {
      if (!isTabDiscarded(nativeTab)) {
        return false;
      }

      return new Promise((resolve, reject) => {
        const timeoutId = setTimeout(() => {
          cleanup();
          reject(new ExtensionError("Tab restore timeout"));
        }, timeout);

        const cleanup = () => {
          clearTimeout(timeoutId);
          nativeTab.removeEventListener("SSTabRestored", onRestored);
        };

        const onRestored = () => {
          cleanup();
          // Wait for Actor initialization
          setTimeout(() => resolve(true), 150);
        };

        nativeTab.addEventListener("SSTabRestored", onRestored);
        lazy.SessionStore.restoreTabContent(nativeTab);
      });
    };

    return {
      nevoflux: {
        // ==================== Tab State ====================

        async getTabState(tabId) {
          const nativeTab = getNativeTab(tabId);
          const browser = getBrowser(nativeTab);
          const discarded = isTabDiscarded(nativeTab);

          return {
            discarded,
            status: discarded ? "unloaded" :
                    browser.webProgress?.isLoadingDocument ? "loading" : "complete",
            url: browser.currentURI?.spec || "",
            title: nativeTab.label || "",
          };
        },

        // ==================== Tab Content ====================

        async getTabContent(tabId, options = {}) {
          const {
            format = "markdown",
            selector = null,
            autoRestore = true,
            keepRestored = false,
            timeout = 30000,
          } = options;

          const nativeTab = getNativeTab(tabId);
          const browser = getBrowser(nativeTab);
          const wasDiscarded = isTabDiscarded(nativeTab);

          // Restore if needed
          if (wasDiscarded) {
            if (!autoRestore) {
              throw new ExtensionError(`Tab ${tabId} is discarded and autoRestore is false`);
            }
            await restoreTabIfNeeded(nativeTab, timeout);
          }

          try {
            const actor = getActor(nativeTab);

            // Use existing getMarkdown action for markdown format
            let result;
            if (format === "markdown") {
              result = await actor.sendQuery("execute", {
                action: "getMarkdown",
                params: { selector },
              });
            } else if (format === "html") {
              result = await actor.sendQuery("execute", {
                action: "getHtml",
                params: { selector: selector || "body" },
              });
              result = { success: true, content: result };
            } else {
              result = await actor.sendQuery("execute", {
                action: "getText",
                params: { selector: selector || "body" },
              });
              result = { success: true, content: result };
            }

            if (!result.success && result.error) {
              throw new ExtensionError(result.error.message || "Content extraction failed");
            }

            return {
              tabId,
              url: browser.currentURI?.spec || "",
              title: nativeTab.label || "",
              content: format === "markdown" ? result.markdown : result.content || result,
              format,
              extractedAt: Date.now(),
              wasDiscarded,
            };
          } finally {
            // Re-discard if it was discarded before
            if (wasDiscarded && !keepRestored) {
              const win = nativeTab.ownerGlobal;
              if (win.gBrowser.selectedTab !== nativeTab) {
                try {
                  win.gBrowser.discardBrowser(nativeTab);
                } catch (e) {
                  // Ignore discard errors
                }
              }
            }
          }
        },

        // ==================== Element Picker (Stub) ====================

        async pickElement(tabId, options = {}) {
          // TODO: Implement in Phase 2
          throw new ExtensionError("pickElement not yet implemented");
        },

        async cancelPicker(tabId) {
          // TODO: Implement in Phase 2
          throw new ExtensionError("cancelPicker not yet implemented");
        },

        // ==================== Selection (Stub) ====================

        async getSelection(tabId) {
          // TODO: Implement in Phase 3
          throw new ExtensionError("getSelection not yet implemented");
        },

        onSelectionChanged: new ExtensionCommon.EventManager({
          context,
          name: "nevoflux.onSelectionChanged",
          register: (fire) => {
            // TODO: Implement in Phase 3
            return () => {};
          },
        }).api(),

        // ==================== Page Lock (Stub) ====================

        async lockPage(tabId, options = {}) {
          // TODO: Implement in Phase 4
          throw new ExtensionError("lockPage not yet implemented");
        },

        async unlockPage(tabId) {
          // TODO: Implement in Phase 4
          throw new ExtensionError("unlockPage not yet implemented");
        },
      },
    };
  }
};
```

**Step 2: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs
git commit -m "feat(agent): add experiment API implementation skeleton"
```

---

### Task 1.3: Update Extension Manifest

**Files:**
- Modify: `src/nevoflux/extensions/nevoflux-agent/manifest.json`

**Step 1: Add experiment_apis section**

Add after the `web_accessible_resources` section (before the closing `}`):

```json
  "experiment_apis": {
    "nevoflux": {
      "schema": "experiment-apis/nevoflux/schema.json",
      "parent": {
        "scopes": ["addon_parent"],
        "paths": [["nevoflux"]],
        "script": "experiment-apis/nevoflux/api.sys.mjs"
      }
    }
  }
```

**Step 2: Verify manifest is valid JSON**

```bash
cat src/nevoflux/extensions/nevoflux-agent/manifest.json | python3 -m json.tool > /dev/null && echo "Valid JSON"
```

**Step 3: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/manifest.json
git commit -m "feat(agent): register experiment API in manifest"
```

---

### Task 1.4: Create JavaScript Bridge for WASM

**Files:**
- Create: `src/nevoflux/extensions/nevoflux-agent/wasm/chat-sidebar/bridge.js`

**Step 1: Create bridge.js**

```javascript
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
```

**Step 2: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/wasm/chat-sidebar/bridge.js
git commit -m "feat(agent): add JavaScript bridge for WASM"
```

---

### Task 1.5: Update WASM init.js to Load Bridge

**Files:**
- Modify: `src/nevoflux/extensions/nevoflux-agent/wasm/chat-sidebar/init.js`

**Step 1: Read current init.js to understand structure**

Run: `cat src/nevoflux/extensions/nevoflux-agent/wasm/chat-sidebar/init.js`

**Step 2: Add bridge loading before WASM initialization**

Add at the beginning of init.js (before WASM loading):

```javascript
// Load NevofluxBridge before WASM
const bridgeScript = document.createElement('script');
bridgeScript.src = 'bridge.js';
document.head.appendChild(bridgeScript);
```

**Step 3: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/wasm/chat-sidebar/init.js
git commit -m "feat(agent): load bridge.js in init.js"
```

---

### Task 1.6: Test Experiment API Registration

**Step 1: Rebuild and reload extension**

```bash
npm run reload-ext
npm run start
```

**Step 2: Open Browser Console and test**

In the browser, press `Ctrl+Shift+J` to open Browser Console, then test:

```javascript
// Test 1: Check if API is registered
typeof browser.nevoflux
// Expected: "object"

// Test 2: Get active tab state
const tabs = await browser.tabs.query({ active: true, currentWindow: true });
await browser.nevoflux.getTabState(tabs[0].id)
// Expected: { discarded: false, status: "complete", url: "...", title: "..." }
```

**Step 3: Document results and commit if needed**

---

## Phase 2: Tab Content with Auto-Restore

### Task 2.1: Test Tab Content Extraction

**Step 1: Test getTabContent with active tab**

```javascript
const tabs = await browser.tabs.query({ active: true, currentWindow: true });
const content = await browser.nevoflux.getTabContent(tabs[0].id, { format: "markdown" });
console.log("Content length:", content.content.length);
console.log("Was discarded:", content.wasDiscarded);
```

**Step 2: Test with discarded tab**

1. Open a new tab, navigate to a page
2. Open another tab
3. Right-click the first tab → "Discard Tab" (or wait for auto-discard)
4. Run:

```javascript
const tabs = await browser.tabs.query({});
const discardedTab = tabs.find(t => t.discarded);
if (discardedTab) {
  const content = await browser.nevoflux.getTabContent(discardedTab.id, { format: "markdown" });
  console.log("Content from discarded tab:", content.content.substring(0, 200));
  console.log("Was discarded:", content.wasDiscarded);
}
```

**Step 3: Verify tab was not switched during restore**

The active tab should remain the same throughout the operation.

---

## Phase 3: Rust Bindings for Dioxus

### Task 3.1: Create Rust Bindings Module

**Files:**
- Create: `src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/bindings/mod.rs`
- Create: `src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/bindings/nevoflux_api.rs`

**Step 1: Create bindings directory**

```bash
mkdir -p src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/bindings
```

**Step 2: Create mod.rs**

```rust
pub mod nevoflux_api;

pub use nevoflux_api::*;
```

**Step 3: Create nevoflux_api.rs**

```rust
//! Rust bindings for browser.nevoflux Experiment API via NevofluxBridge

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// ==================== Type Definitions ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabContent {
    #[serde(rename = "tabId")]
    pub tab_id: u32,
    pub url: String,
    pub title: String,
    pub content: String,
    pub format: String,
    #[serde(rename = "extractedAt")]
    pub extracted_at: u64,
    #[serde(rename = "wasDiscarded")]
    pub was_discarded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub discarded: bool,
    pub status: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub active: bool,
    pub discarded: bool,
    #[serde(rename = "favIconUrl")]
    pub fav_icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickerResult {
    pub selector: String,
    pub xpath: String,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    pub id: Option<String>,
    #[serde(rename = "className")]
    pub class_name: Option<String>,
    pub text: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
    pub rect: ElementRect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRect {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionData {
    pub text: String,
    pub html: String,
    pub rect: ElementRect,
    #[serde(rename = "anchorNode")]
    pub anchor_node: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContentOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "autoRestore")]
    pub auto_restore: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "keepRestored")]
    pub keep_restored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

// ==================== JS Bridge Bindings ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = isAvailable)]
    fn js_is_available() -> bool;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = getTabContent, catch)]
    async fn js_get_tab_content(tab_id: u32, options_json: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = getTabState, catch)]
    async fn js_get_tab_state(tab_id: u32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = getAllTabs, catch)]
    async fn js_get_all_tabs() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = getActiveTab, catch)]
    async fn js_get_active_tab() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = pickElement, catch)]
    async fn js_pick_element(tab_id: u32, options_json: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = cancelPicker, catch)]
    async fn js_cancel_picker(tab_id: u32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = getSelection, catch)]
    async fn js_get_selection(tab_id: u32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = lockPage, catch)]
    async fn js_lock_page(tab_id: u32, options_json: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = NevofluxBridge, js_name = unlockPage, catch)]
    async fn js_unlock_page(tab_id: u32) -> Result<JsValue, JsValue>;
}

// ==================== API Result Handling ====================

#[derive(Debug, Deserialize)]
struct ApiResult<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

fn parse_result<T: for<'de> Deserialize<'de>>(js_value: JsValue) -> Result<T, String> {
    let json_str = js_value
        .as_string()
        .ok_or_else(|| "Response is not a string".to_string())?;

    let result: ApiResult<T> = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    if result.success {
        result.data.ok_or_else(|| "No data in response".to_string())
    } else {
        Err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

// ==================== Public API ====================

/// Check if NevofluxBridge is available
pub fn is_available() -> bool {
    js_is_available()
}

/// Get tab content as markdown/html/text (auto-restores discarded tabs)
pub async fn get_tab_content(tab_id: u32, options: Option<GetContentOptions>) -> Result<TabContent, String> {
    let options_json = match options {
        Some(opts) => serde_json::to_string(&opts).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    };

    let result = js_get_tab_content(tab_id, &options_json)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Get tab state (discarded, loading, complete)
pub async fn get_tab_state(tab_id: u32) -> Result<TabState, String> {
    let result = js_get_tab_state(tab_id)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Get all tabs in the current window
pub async fn get_all_tabs() -> Result<Vec<TabInfo>, String> {
    let result = js_get_all_tabs()
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Get current active tab
pub async fn get_active_tab() -> Result<TabInfo, String> {
    let result = js_get_active_tab()
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Start element picker and wait for user selection
pub async fn pick_element(tab_id: u32, hint: Option<&str>) -> Result<PickerResult, String> {
    let options = if let Some(h) = hint {
        format!(r#"{{"hint":"{}"}}"#, h)
    } else {
        "{}".to_string()
    };

    let result = js_pick_element(tab_id, &options)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Cancel active element picker
pub async fn cancel_picker(tab_id: u32) -> Result<(), String> {
    js_cancel_picker(tab_id)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    Ok(())
}

/// Get current text selection from a tab
pub async fn get_selection(tab_id: u32) -> Result<Option<SelectionData>, String> {
    let result = js_get_selection(tab_id)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    parse_result(result)
}

/// Lock page to prevent user interaction
pub async fn lock_page(tab_id: u32, message: Option<&str>) -> Result<(), String> {
    let options = if let Some(m) = message {
        format!(r#"{{"showOverlay":true,"message":"{}"}}"#, m)
    } else {
        r#"{"showOverlay":true}"#.to_string()
    };

    js_lock_page(tab_id, &options)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    Ok(())
}

/// Unlock page after agent operations
pub async fn unlock_page(tab_id: u32) -> Result<(), String> {
    js_unlock_page(tab_id)
        .await
        .map_err(|e| format!("JS error: {:?}", e))?;

    Ok(())
}
```

**Step 4: Update lib.rs to include bindings module**

Add to `src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/lib.rs`:

```rust
pub mod bindings;
```

**Step 5: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/bindings/
git add src/nevoflux/extensions/nevoflux-agent/dioxus-ui/chat-sidebar/src/lib.rs
git commit -m "feat(agent): add Rust bindings for nevoflux bridge"
```

---

## Phase 4: Element Picker Implementation

### Task 4.1: Add Picker Methods to NevofluxChild Actor

**Files:**
- Modify: `src/nevoflux/engine-overlays/browser/actors/NevofluxChild.sys.mjs`

**Step 1: Add picker handler to receiveMessage**

In the `receiveMessage` method, add cases:

```javascript
case "startPicker":
  return this.startPicker(data);
case "stopPicker":
  return this.stopPicker();
case "getSelection":
  return this.getCurrentSelection();
```

**Step 2: Add picker implementation methods**

Add these methods to the NevofluxChild class:

```javascript
// ========== Element Picker ==========

startPicker({ filter = "any", highlightColor = "#6366f1" }) {
  if (this._pickerActive) {
    return { success: false, error: "Picker already active" };
  }

  this._pickerActive = true;
  this._pickerFilter = filter;
  this._highlightColor = highlightColor;
  this._pickerResolve = null;
  this._pickerReject = null;

  this._createPickerHighlight();

  this.doc.addEventListener("mousemove", this._onPickerMove, true);
  this.doc.addEventListener("click", this._onPickerClick, true);
  this.doc.addEventListener("keydown", this._onPickerKey, true);

  this._originalCursor = this.doc.body.style.cursor;
  this.doc.body.style.cursor = "crosshair";

  return new Promise((resolve, reject) => {
    this._pickerResolve = resolve;
    this._pickerReject = reject;
  });
}

stopPicker() {
  if (!this._pickerActive) return { success: true };

  this._pickerActive = false;

  this.doc.removeEventListener("mousemove", this._onPickerMove, true);
  this.doc.removeEventListener("click", this._onPickerClick, true);
  this.doc.removeEventListener("keydown", this._onPickerKey, true);

  this.doc.body.style.cursor = this._originalCursor || "";
  this._removePickerHighlight();

  if (this._pickerReject) {
    this._pickerReject({ success: false, error: "cancelled" });
  }

  return { success: true };
}

_createPickerHighlight() {
  if (this._highlightEl) return;

  this._highlightEl = this.doc.createElement("div");
  this._highlightEl.id = "nevoflux-picker-highlight";
  this._highlightEl.style.cssText = `
    position: fixed;
    pointer-events: none;
    z-index: 2147483647;
    border: 2px solid ${this._highlightColor};
    background: ${this._highlightColor}20;
    border-radius: 3px;
    transition: all 0.1s ease-out;
    display: none;
  `;

  this._labelEl = this.doc.createElement("div");
  this._labelEl.style.cssText = `
    position: absolute;
    bottom: 100%;
    left: 0;
    background: ${this._highlightColor};
    color: white;
    font-size: 11px;
    font-family: system-ui, sans-serif;
    padding: 2px 6px;
    border-radius: 3px 3px 0 0;
    white-space: nowrap;
  `;
  this._highlightEl.appendChild(this._labelEl);

  this.doc.body.appendChild(this._highlightEl);
}

_removePickerHighlight() {
  this._highlightEl?.remove();
  this._highlightEl = null;
  this._labelEl = null;
  this._hoveredEl = null;
}

_onPickerMove = (event) => {
  event.stopPropagation();

  let target = event.target;
  if (target === this._highlightEl || this._highlightEl?.contains(target)) {
    return;
  }

  this._hoveredEl = target;
  this._updatePickerHighlight(target);
};

_onPickerClick = (event) => {
  event.stopPropagation();
  event.preventDefault();

  const target = this._hoveredEl;
  if (!target) return;

  const result = {
    selector: this._generateStableSelector(target),
    xpath: this._generateXPath(target),
    tagName: target.tagName.toLowerCase(),
    id: target.id || null,
    className: typeof target.className === "string" ? target.className : null,
    text: target.textContent?.slice(0, 200)?.trim() || null,
    attributes: this._getElementAttributes(target),
    rect: target.getBoundingClientRect().toJSON(),
  };

  this.stopPicker();

  if (this._pickerResolve) {
    this._pickerResolve({ success: true, data: result });
  }
};

_onPickerKey = (event) => {
  event.stopPropagation();

  if (event.key === "Escape") {
    event.preventDefault();
    this.stopPicker();
  }
};

_updatePickerHighlight(element) {
  if (!this._highlightEl || !element) return;

  const rect = element.getBoundingClientRect();

  this._highlightEl.style.display = "block";
  this._highlightEl.style.top = `${rect.top}px`;
  this._highlightEl.style.left = `${rect.left}px`;
  this._highlightEl.style.width = `${rect.width}px`;
  this._highlightEl.style.height = `${rect.height}px`;

  const tag = element.tagName.toLowerCase();
  const id = element.id ? `#${element.id}` : "";
  const cls = element.className && typeof element.className === "string"
    ? `.${element.className.split(" ")[0]}`
    : "";
  this._labelEl.textContent = `${tag}${id}${cls}`;
}

_generateStableSelector(element) {
  if (!element || element === this.doc.body) return "body";

  // Priority 1: Unique ID
  if (element.id) {
    const selector = `#${CSS.escape(element.id)}`;
    if (this.doc.querySelectorAll(selector).length === 1) {
      return selector;
    }
  }

  // Priority 2: data-testid or data-* attributes
  for (const attr of element.attributes) {
    if (attr.name === "data-testid" || attr.name.startsWith("data-")) {
      const selector = `[${attr.name}="${CSS.escape(attr.value)}"]`;
      if (this.doc.querySelectorAll(selector).length === 1) {
        return selector;
      }
    }
  }

  // Priority 3: Build path
  const path = [];
  let current = element;

  while (current && current !== this.doc.body) {
    let selector = current.tagName.toLowerCase();

    if (current.id) {
      path.unshift(`#${CSS.escape(current.id)}`);
      break;
    }

    const parent = current.parentElement;
    if (parent) {
      const siblings = Array.from(parent.children).filter(
        el => el.tagName === current.tagName
      );
      if (siblings.length > 1) {
        const index = siblings.indexOf(current) + 1;
        selector += `:nth-of-type(${index})`;
      }
    }

    path.unshift(selector);
    current = current.parentElement;
  }

  return path.join(" > ");
}

_generateXPath(element) {
  if (!element) return "";

  const parts = [];
  let current = element;

  while (current && current.nodeType === Node.ELEMENT_NODE) {
    let index = 1;
    let sibling = current.previousElementSibling;

    while (sibling) {
      if (sibling.tagName === current.tagName) index++;
      sibling = sibling.previousElementSibling;
    }

    const tagName = current.tagName.toLowerCase();
    parts.unshift(`${tagName}[${index}]`);
    current = current.parentElement;
  }

  return "/" + parts.join("/");
}

_getElementAttributes(element) {
  const attrs = {};
  for (const attr of element.attributes) {
    if (attr.value.length < 200 && !attr.name.startsWith("on")) {
      attrs[attr.name] = attr.value;
    }
  }
  return attrs;
}

// ========== Selection ==========

getCurrentSelection() {
  const selection = this.win.getSelection();

  if (!selection || selection.isCollapsed) {
    return { success: true, data: null };
  }

  const text = selection.toString().trim();
  if (!text) {
    return { success: true, data: null };
  }

  const range = selection.getRangeAt(0);
  const rect = range.getBoundingClientRect();
  const container = this.doc.createElement("div");
  container.appendChild(range.cloneContents());

  return {
    success: true,
    data: {
      text,
      html: container.innerHTML,
      rect: rect.toJSON(),
      anchorNode: this._generateStableSelector(selection.anchorNode.parentElement),
      url: this.win.location.href,
      title: this.doc.title,
    },
  };
}
```

**Step 3: Commit**

```bash
git add src/nevoflux/engine-overlays/browser/actors/NevofluxChild.sys.mjs
git commit -m "feat(agent): add element picker and selection methods to Child actor"
```

---

### Task 4.2: Update Experiment API for Picker

**Files:**
- Modify: `src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs`

**Step 1: Replace pickElement stub with implementation**

```javascript
async pickElement(tabId, options = {}) {
  const {
    hint = "",
    filter = "any",
    timeout = 60000,
    highlightColor = "#6366f1",
  } = options;

  const nativeTab = getNativeTab(tabId);

  // Restore tab if needed
  await restoreTabIfNeeded(nativeTab, 30000);

  const actor = getActor(nativeTab);

  // Start picker with timeout
  const timeoutPromise = new Promise((_, reject) => {
    setTimeout(() => reject(new ExtensionError("Picker timeout")), timeout);
  });

  const pickerPromise = actor.sendQuery("startPicker", {
    filter,
    highlightColor,
  });

  try {
    const result = await Promise.race([pickerPromise, timeoutPromise]);
    if (!result.success) {
      throw new ExtensionError(result.error || "Picker failed");
    }
    return result.data;
  } catch (e) {
    // Ensure picker is stopped on error
    try {
      await actor.sendQuery("stopPicker", {});
    } catch {}
    throw e;
  }
},

async cancelPicker(tabId) {
  const nativeTab = getNativeTab(tabId);
  const actor = getActor(nativeTab);
  await actor.sendQuery("stopPicker", {});
},
```

**Step 2: Replace getSelection stub with implementation**

```javascript
async getSelection(tabId) {
  const nativeTab = getNativeTab(tabId);

  if (isTabDiscarded(nativeTab)) {
    return null;
  }

  const actor = getActor(nativeTab);
  const result = await actor.sendQuery("getSelection", {});

  return result.success ? result.data : null;
},
```

**Step 3: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs
git commit -m "feat(agent): implement picker and selection in experiment API"
```

---

### Task 4.3: Test Element Picker

**Step 1: Rebuild extension**

```bash
npm run reload-ext
npm run start
```

**Step 2: Test picker in Browser Console**

```javascript
const tabs = await browser.tabs.query({ active: true, currentWindow: true });
const result = await browser.nevoflux.pickElement(tabs[0].id, { hint: "Select an element" });
console.log("Picked:", result);
```

**Step 3: Verify**

- Cursor should change to crosshair
- Elements should highlight on hover
- Clicking should return selector info
- Pressing Escape should cancel

---

## Phase 5: Page Lock Implementation

### Task 5.1: Add Lock Methods to NevofluxChild Actor

**Files:**
- Modify: `src/nevoflux/engine-overlays/browser/actors/NevofluxChild.sys.mjs`

**Step 1: Add lock cases to receiveMessage**

```javascript
case "lockPage":
  return this.lockPage(data);
case "unlockPage":
  return this.unlockPage();
```

**Step 2: Add lock implementation**

```javascript
// ========== Page Lock ==========

lockPage({ showOverlay = true, message = "" }) {
  if (this._pageLocked) return { success: true };

  this._pageLocked = true;

  // Event locking
  this._lockHandler = (event) => {
    event.stopImmediatePropagation();
    event.preventDefault();
  };

  const events = [
    "mousedown", "mouseup", "click", "dblclick", "contextmenu",
    "keydown", "keyup", "keypress",
    "touchstart", "touchend", "touchmove",
    "wheel", "scroll",
  ];

  events.forEach(type => {
    this.doc.addEventListener(type, this._lockHandler, { capture: true });
  });

  this._lockEvents = events;

  // Visual overlay
  if (showOverlay) {
    this._createLockOverlay(message);
  }

  return { success: true };
}

unlockPage() {
  if (!this._pageLocked) return { success: true };

  this._pageLocked = false;

  if (this._lockHandler && this._lockEvents) {
    this._lockEvents.forEach(type => {
      this.doc.removeEventListener(type, this._lockHandler, { capture: true });
    });
  }
  this._lockHandler = null;
  this._lockEvents = null;

  this._removeLockOverlay();

  return { success: true };
}

_createLockOverlay(message) {
  if (this._lockOverlay) return;

  this._lockOverlay = this.doc.createElement("div");
  this._lockOverlay.id = "nevoflux-lock-overlay";
  this._lockOverlay.innerHTML = `
    <div style="
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 16px;
    ">
      <div style="
        width: 48px;
        height: 48px;
        border: 3px solid rgba(255,255,255,0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: nevoflux-spin 1s linear infinite;
      "></div>
      <div style="
        color: white;
        font-size: 14px;
        font-family: system-ui, sans-serif;
      ">${message || "Agent working..."}</div>
    </div>
    <style>
      @keyframes nevoflux-spin {
        to { transform: rotate(360deg); }
      }
    </style>
  `;
  this._lockOverlay.style.cssText = `
    position: fixed;
    inset: 0;
    z-index: 2147483646;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  `;

  this.doc.body.appendChild(this._lockOverlay);
}

_removeLockOverlay() {
  this._lockOverlay?.remove();
  this._lockOverlay = null;
}
```

**Step 3: Commit**

```bash
git add src/nevoflux/engine-overlays/browser/actors/NevofluxChild.sys.mjs
git commit -m "feat(agent): add page lock methods to Child actor"
```

---

### Task 5.2: Update Experiment API for Page Lock

**Files:**
- Modify: `src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs`

**Step 1: Replace lock stubs with implementation**

```javascript
async lockPage(tabId, options = {}) {
  const { showOverlay = true, message = "" } = options;

  const nativeTab = getNativeTab(tabId);
  const actor = getActor(nativeTab);

  await actor.sendQuery("lockPage", { showOverlay, message });
},

async unlockPage(tabId) {
  const nativeTab = getNativeTab(tabId);
  const actor = getActor(nativeTab);

  await actor.sendQuery("unlockPage", {});
},
```

**Step 2: Commit**

```bash
git add src/nevoflux/extensions/nevoflux-agent/experiment-apis/nevoflux/api.sys.mjs
git commit -m "feat(agent): implement page lock in experiment API"
```

---

### Task 5.3: Test Page Lock

```javascript
const tabs = await browser.tabs.query({ active: true, currentWindow: true });

// Lock
await browser.nevoflux.lockPage(tabs[0].id, { message: "Testing lock..." });
// Try clicking on the page - should be blocked

// Unlock after 3 seconds
setTimeout(async () => {
  await browser.nevoflux.unlockPage(tabs[0].id);
  console.log("Unlocked");
}, 3000);
```

---

## Phase 6: Integration Test

### Task 6.1: Full Integration Test

**Step 1: Rebuild everything**

```bash
npm run reload-ext
cd src/nevoflux/extensions/nevoflux-agent/dioxus-ui && ./build.sh
npm run reload-ext
npm run start
```

**Step 2: Test complete flow**

```javascript
// 1. Get tab state
const tabs = await browser.tabs.query({ active: true, currentWindow: true });
const state = await browser.nevoflux.getTabState(tabs[0].id);
console.log("State:", state);

// 2. Get content
const content = await browser.nevoflux.getTabContent(tabs[0].id, { format: "markdown" });
console.log("Content preview:", content.content.substring(0, 500));

// 3. Pick element
console.log("Click on any element...");
const picked = await browser.nevoflux.pickElement(tabs[0].id);
console.log("Picked:", picked);

// 4. Lock/unlock
await browser.nevoflux.lockPage(tabs[0].id, { message: "Test" });
setTimeout(() => browser.nevoflux.unlockPage(tabs[0].id), 2000);
```

**Step 3: Test with WASM bridge**

Open sidebar and verify NevofluxBridge is available:

```javascript
// In sidebar context (via devtools on sidebar)
NevofluxBridge.isAvailable()
// Should return true

NevofluxBridge.getActiveTab().then(console.log)
// Should return active tab info
```

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Phase 1 | Experiment API Foundation | Pending |
| Phase 2 | Tab Content with Auto-Restore | Pending |
| Phase 3 | Rust Bindings for Dioxus | Pending |
| Phase 4 | Element Picker Implementation | Pending |
| Phase 5 | Page Lock Implementation | Pending |
| Phase 6 | Integration Test | Pending |

Total estimated tasks: 15
