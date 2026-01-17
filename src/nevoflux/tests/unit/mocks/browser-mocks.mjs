/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/**
 * Mock implementations for Firefox browser APIs used in NevoFlux P1
 */

// Mock localStorage/sessionStorage
export function createMockStorage() {
  const storage = new Map();
  return {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, String(value)),
    removeItem: (key) => storage.delete(key),
    clear: () => storage.clear(),
    get length() { return storage.size; },
    key: (index) => {
      const keys = Array.from(storage.keys());
      return keys[index] ?? null;
    },
  };
}

// Mock document
export function createMockDocument() {
  const elements = new Map();
  let elementCounter = 0;

  const createElement = (tagName) => {
    const el = {
      tagName: tagName.toUpperCase(),
      id: '',
      className: '',
      textContent: '',
      innerHTML: '',
      value: '',
      style: {},
      children: [],
      parentElement: null,
      attributes: new Map(),
      _eventListeners: new Map(),

      querySelector: (selector) => {
        if (selector.startsWith('#')) {
          return elements.get(selector.slice(1)) || null;
        }
        return null;
      },
      querySelectorAll: () => [],
      appendChild: function(child) {
        this.children.push(child);
        child.parentElement = this;
        return child;
      },
      prepend: function(child) {
        this.children.unshift(child);
        child.parentElement = this;
      },
      remove: function() {
        if (this.parentElement) {
          const idx = this.parentElement.children.indexOf(this);
          if (idx > -1) this.parentElement.children.splice(idx, 1);
        }
        if (this.id) elements.delete(this.id);
      },
      focus: () => {},
      blur: () => {},
      click: () => {},
      scrollIntoView: () => {},
      getBoundingClientRect: () => ({
        left: 0, top: 0, right: 100, bottom: 100,
        width: 100, height: 100, x: 0, y: 0,
      }),
      getAttribute: function(name) {
        return this.attributes.get(name) ?? null;
      },
      setAttribute: function(name, value) {
        this.attributes.set(name, value);
        if (name === 'id') {
          this.id = value;
          elements.set(value, this);
        }
      },
      addEventListener: function(event, handler) {
        if (!this._eventListeners.has(event)) {
          this._eventListeners.set(event, []);
        }
        this._eventListeners.get(event).push(handler);
      },
      dispatchEvent: function(event) {
        const handlers = this._eventListeners.get(event.type) || [];
        handlers.forEach(h => h(event));
        return true;
      },
    };

    // Auto-register if id is set
    Object.defineProperty(el, 'id', {
      get: () => el._id || '',
      set: (value) => {
        if (el._id) elements.delete(el._id);
        el._id = value;
        if (value) elements.set(value, el);
      },
    });

    return el;
  };

  const body = createElement('body');
  const documentElement = createElement('html');
  documentElement.appendChild(body);

  return {
    body,
    documentElement,
    createElement,
    getElementById: (id) => elements.get(id) || null,
    querySelector: (selector) => {
      if (selector === 'body') return body;
      if (selector.startsWith('#')) {
        return elements.get(selector.slice(1)) || null;
      }
      return null;
    },
    querySelectorAll: () => [],
    defaultView: null, // Will be set by window mock
  };
}

// Mock window
export function createMockWindow(doc) {
  const localStorage = createMockStorage();
  const sessionStorage = createMockStorage();

  const win = {
    document: doc,
    localStorage,
    sessionStorage,
    innerWidth: 1920,
    innerHeight: 1080,
    scrollX: 0,
    scrollY: 0,
    setTimeout: (fn, ms) => setTimeout(fn, ms),
    clearTimeout: (id) => clearTimeout(id),
    getComputedStyle: () => ({
      visibility: 'visible',
      display: 'block',
      opacity: '1',
    }),
    eval: (script) => {
      // eslint-disable-next-line no-eval
      return eval(script);
    },
    windowUtils: {
      sendKeyEvent: (type, keyCode, charCode, modifiers) => {},
      sendMouseEvent: (type, x, y, button, clickCount, modifiers) => {},
      sendWheelEvent: (x, y, dx, dy, dz, mode, lineX, lineY, options) => {},
    },
    MouseEvent: class MockMouseEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.bubbles = init.bubbles ?? false;
        this.cancelable = init.cancelable ?? false;
        this.clientX = init.clientX ?? 0;
        this.clientY = init.clientY ?? 0;
        this.button = init.button ?? 0;
        this.view = init.view ?? null;
      }
    },
    Event: class MockEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.bubbles = init.bubbles ?? false;
        this.cancelable = init.cancelable ?? false;
      }
    },
  };

  doc.defaultView = win;
  return win;
}

// Mock Services (Firefox privileged API)
export function createMockServices() {
  const cookies = [];

  return {
    cookies: {
      cookies: {
        [Symbol.iterator]: function* () {
          for (const cookie of cookies) {
            yield cookie;
          }
        },
      },
      add: (domain, path, name, value, secure, httpOnly, isSession, expiry, origin, sameSite, scheme) => {
        cookies.push({
          host: domain,
          path,
          name,
          value,
          isSecure: secure,
          isHttpOnly: httpOnly,
          isSession,
          expiry,
          sameSite,
        });
      },
      remove: (domain, name, path) => {
        const idx = cookies.findIndex(c => c.host === domain && c.name === name && c.path === path);
        if (idx > -1) cookies.splice(idx, 1);
      },
      removeAll: () => {
        cookies.length = 0;
      },
      _getAll: () => [...cookies], // Test helper
    },
    io: {
      newURI: (url) => ({ spec: url }),
    },
    scriptSecurityManager: {
      getSystemPrincipal: () => ({}),
    },
    ww: {
      openWindow: (parent, url, name, features) => ({
        windowId: Date.now(),
      }),
    },
  };
}

// Mock tab/window managers
export function createMockTabManager() {
  const tabs = new Map();
  let tabIdCounter = 1;

  return {
    get: (tabId) => tabs.get(tabId) || null,
    add: (url = 'about:newtab') => {
      const tabId = tabIdCounter++;
      const tab = {
        id: tabId,
        nativeTab: {
          _tPos: tabs.size,
          ownerGlobal: { gBrowser: { selectedTab: null, tabs: [] } },
          linkedBrowser: {
            webProgress: { isLoadingDocument: false },
          },
        },
        browser: {
          currentURI: { spec: url },
          contentTitle: 'Mock Tab',
        },
      };
      tabs.set(tabId, tab);
      return tab;
    },
    remove: (tabId) => tabs.delete(tabId),
    getAll: () => Array.from(tabs.values()),
    _tabs: tabs,
  };
}

export function createMockWindowManager() {
  const windows = new Map();
  let windowIdCounter = 1;

  return {
    topWindow: { gBrowser: { tabs: [], addTab: () => {}, selectedTab: null } },
    get: (windowId) => windows.get(windowId) || null,
    getWrapper: (win) => ({
      id: windowIdCounter,
      window: win || { gBrowser: { tabs: [], addTab: () => {}, selectedTab: null } },
    }),
    add: () => {
      const winId = windowIdCounter++;
      const wrapper = {
        id: winId,
        window: {
          gBrowser: { tabs: [], addTab: () => {}, selectedTab: null },
          close: () => windows.delete(winId),
          focus: () => {},
        },
      };
      windows.set(winId, wrapper);
      return wrapper;
    },
    _windows: windows,
  };
}

// Mock tabTracker
export function createMockTabTracker() {
  const tabIds = new WeakMap();
  let idCounter = 1;

  return {
    activeTab: null,
    getId: (tab) => {
      if (!tabIds.has(tab)) {
        tabIds.set(tab, idCounter++);
      }
      return tabIds.get(tab);
    },
  };
}

// Mock extension context
export function createMockExtension() {
  const tabManager = createMockTabManager();
  const windowManager = createMockWindowManager();

  return {
    tabManager,
    windowManager,
    context: {},
    apiManager: {
      getAPI: () => null,
    },
  };
}

// Mock ChromeUtils
export const MockChromeUtils = {
  importESModule: (module) => {
    if (module.includes('Timer.sys.mjs')) {
      return {
        setTimeout: (fn, ms) => setTimeout(fn, ms),
        clearTimeout: (id) => clearTimeout(id),
      };
    }
    return {};
  },
  generateQI: (interfaces) => () => null,
};

// Mock Ci (Component interfaces)
export const MockCi = {
  nsICookie: {
    SCHEME_HTTPS: 1,
  },
  nsIWebProgressListener: {
    STATE_STOP: 0x10,
    STATE_IS_NETWORK: 0x40000,
  },
  nsIWebProgress: {
    NOTIFY_STATE_ALL: 0xff,
  },
};
