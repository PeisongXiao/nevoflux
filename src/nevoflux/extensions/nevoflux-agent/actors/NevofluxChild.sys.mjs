/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

export class NevofluxChild extends JSWindowActorChild {
  receiveMessage({ name, data }) {
    if (name === "execute") {
      return this.execute(data.action, data.params);
    }
    return null;
  }

  async execute(action, params) {
    const handlers = {
      getText: () => this.getText(params),
      getHtml: () => this.getHtml(params),
      getValue: () => this.getValue(params),
      snapshot: () => this.snapshot(params),
      screenshot: () => this.screenshot(params),
      isVisible: () => this.isVisible(params),
      exists: () => this.exists(params),
      click: () => this.click(params),
      type: () => this.type(params),
      fill: () => this.fill(params),
      waitForSelector: () => this.waitForSelector(params),
    };

    const handler = handlers[action];
    if (!handler) {
      return { success: false, error: { code: 5002, message: `Unknown action: ${action}`, recoverable: false } };
    }

    try {
      return await handler();
    } catch (e) {
      return { success: false, error: { code: 5001, message: e.message, recoverable: false } };
    }
  }

  // ========== Data Extraction ==========

  getText({ selector }) {
    const el = this.document.querySelector(selector);
    return el?.textContent || "";
  }

  getHtml({ selector }) {
    const el = this.document.querySelector(selector);
    return el?.innerHTML || "";
  }

  getValue({ selector }) {
    const el = this.document.querySelector(selector);
    return el?.value || "";
  }

  snapshot({ interactive = false, compact = false, depth, root = "body" }) {
    const rootEl = this.document.querySelector(root);
    if (!rootEl) {
      return { tree: "", refs: {} };
    }

    const refs = {};
    let refCounter = 1;

    const buildTree = (el, currentDepth = 0) => {
      if (depth !== undefined && currentDepth > depth) {
        return "";
      }

      const role = this.inferRole(el);
      const name = this.getAccessibleName(el);

      // Filter: only interactive elements if interactive=true
      if (interactive && !this.isInteractive(el)) {
        return Array.from(el.children)
          .map(c => buildTree(c, currentDepth))
          .filter(Boolean)
          .join("");
      }

      // Filter: skip empty elements if compact=true
      if (compact && !this.hasContent(el) && !this.isInteractive(el)) {
        return Array.from(el.children)
          .map(c => buildTree(c, currentDepth))
          .filter(Boolean)
          .join("");
      }

      const refId = `e${refCounter++}`;
      refs[refId] = {
        role,
        name: name || "",
        selector: this.generateSelector(el),
        tagName: el.tagName.toLowerCase(),
      };

      const indent = "  ".repeat(currentDepth);
      const children = Array.from(el.children)
        .map(c => buildTree(c, currentDepth + 1))
        .filter(Boolean)
        .join("");

      const nameStr = name ? ` "${name}"` : "";
      return `${indent}- ${role}${nameStr} [ref=${refId}]\n${children}`;
    };

    return {
      tree: buildTree(rootEl),
      refs,
    };
  }

  async screenshot({ fullPage = false, type = "png", quality = 80 }) {
    // Use canvas to capture screenshot
    const win = this.contentWindow;
    const doc = this.document;

    const width = fullPage ? doc.documentElement.scrollWidth : win.innerWidth;
    const height = fullPage ? doc.documentElement.scrollHeight : win.innerHeight;

    const canvas = doc.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");

    // For full page, we need to scroll and capture
    if (fullPage) {
      const viewportHeight = win.innerHeight;
      const chunks = Math.ceil(height / viewportHeight);

      for (let i = 0; i < chunks; i++) {
        const scrollY = i * viewportHeight;
        win.scrollTo(0, scrollY);
        await this.sleep(50); // Wait for render

        // Note: drawWindow is privileged and may not work in content process
        // This is a simplified implementation
      }
    }

    // Fallback: return placeholder for now
    // Real implementation would use browser's screenshot API
    const mimeType = type === "jpeg" ? "image/jpeg" : "image/png";

    return {
      image: "", // Base64 placeholder - real impl uses privileged API
      mimeType,
      width,
      height,
    };
  }

  // ========== State Checking ==========

  isVisible({ selector }) {
    const el = this.document.querySelector(selector);
    if (!el) return false;

    const rect = el.getBoundingClientRect();
    const style = this.contentWindow.getComputedStyle(el);

    return (
      rect.width > 0 &&
      rect.height > 0 &&
      style.visibility !== "hidden" &&
      style.display !== "none" &&
      style.opacity !== "0"
    );
  }

  exists({ selector }) {
    return this.document.querySelector(selector) !== null;
  }

  // ========== Interaction ==========

  async click({ selector, button = "left", clickCount = 1, delay = 0, force = false }) {
    const el = this.document.querySelector(selector);
    if (!el) {
      return { success: false, error: { code: 1001, message: "Element not found", recoverable: true, suggestion: "Use waitForSelector first" } };
    }

    if (!force && !this.isVisible({ selector })) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      await this.sleep(300);

      if (!this.isVisible({ selector })) {
        return { success: false, error: { code: 1002, message: "Element not visible", recoverable: true, suggestion: "Use force: true to click anyway" } };
      }
    }

    const rect = el.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const buttonCode = { left: 0, middle: 1, right: 2 }[button] || 0;

    const eventInit = {
      bubbles: true,
      cancelable: true,
      view: this.contentWindow,
      clientX: x,
      clientY: y,
      button: buttonCode,
    };

    for (let i = 0; i < clickCount; i++) {
      el.dispatchEvent(new MouseEvent("mouseover", eventInit));
      await this.sleep(10);
      el.dispatchEvent(new MouseEvent("mousedown", eventInit));
      await this.sleep(50 + Math.random() * 30);
      el.dispatchEvent(new MouseEvent("mouseup", eventInit));
      el.dispatchEvent(new MouseEvent("click", eventInit));

      if (delay > 0 && i < clickCount - 1) {
        await this.sleep(delay);
      }
    }

    return { success: true };
  }

  async type({ selector, text, delay = 50 }) {
    const el = this.document.querySelector(selector);
    if (!el) {
      return { success: false, error: { code: 1001, message: "Element not found", recoverable: true } };
    }

    el.focus();

    for (const char of text) {
      const eventInit = {
        bubbles: true,
        cancelable: true,
        key: char,
        code: `Key${char.toUpperCase()}`,
        charCode: char.charCodeAt(0),
      };

      el.dispatchEvent(new KeyboardEvent("keydown", eventInit));
      el.dispatchEvent(new KeyboardEvent("keypress", eventInit));

      if (el.tagName === "INPUT" || el.tagName === "TEXTAREA") {
        el.value += char;
        el.dispatchEvent(new Event("input", { bubbles: true }));
      }

      el.dispatchEvent(new KeyboardEvent("keyup", eventInit));

      await this.sleep(delay + Math.random() * delay * 0.5);
    }

    return { success: true };
  }

  fill({ selector, text }) {
    const el = this.document.querySelector(selector);
    if (!el) {
      return { success: false, error: { code: 1001, message: "Element not found", recoverable: true } };
    }

    el.focus();
    el.value = "";
    el.value = text;
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));

    return { success: true };
  }

  // ========== Wait ==========

  async waitForSelector({ selector, timeout = 30000, state = "visible" }) {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      const el = this.document.querySelector(selector);

      const stateChecks = {
        attached: () => el !== null,
        detached: () => el === null,
        visible: () => el && this.isVisible({ selector }),
        hidden: () => !el || !this.isVisible({ selector }),
      };

      if (stateChecks[state]?.()) {
        return { success: true };
      }

      await this.sleep(100);
    }

    return { success: false, error: { code: 4001, message: `Timeout waiting for ${selector}`, recoverable: true } };
  }

  // ========== Helpers ==========

  sleep(ms) {
    return new Promise(resolve => this.contentWindow.setTimeout(resolve, ms));
  }

  inferRole(el) {
    const roleMap = {
      A: "link",
      BUTTON: "button",
      INPUT: "textbox",
      SELECT: "combobox",
      TEXTAREA: "textbox",
      IMG: "image",
      H1: "heading",
      H2: "heading",
      H3: "heading",
      H4: "heading",
      H5: "heading",
      H6: "heading",
      NAV: "navigation",
      MAIN: "main",
      ASIDE: "complementary",
      FOOTER: "contentinfo",
      HEADER: "banner",
      FORM: "form",
      TABLE: "table",
      UL: "list",
      OL: "list",
      LI: "listitem",
    };
    return el.getAttribute("role") || roleMap[el.tagName] || "generic";
  }

  getAccessibleName(el) {
    return (
      el.getAttribute("aria-label") ||
      el.getAttribute("alt") ||
      el.getAttribute("title") ||
      (el.tagName === "INPUT" ? el.getAttribute("placeholder") : null) ||
      (el.textContent?.trim().slice(0, 50) || null)
    );
  }

  isInteractive(el) {
    const interactiveTags = ["A", "BUTTON", "INPUT", "SELECT", "TEXTAREA"];
    const hasClickHandler = el.onclick !== null;
    const hasRole = ["button", "link", "textbox", "checkbox", "radio", "combobox"].includes(
      el.getAttribute("role")
    );
    const isTabFocusable = el.getAttribute("tabindex") !== null;

    return interactiveTags.includes(el.tagName) || hasClickHandler || hasRole || isTabFocusable;
  }

  hasContent(el) {
    return el.textContent?.trim().length > 0 || el.querySelector("img, video, canvas, svg");
  }

  generateSelector(el) {
    if (el.id) {
      return `#${CSS.escape(el.id)}`;
    }

    const path = [];
    let current = el;

    while (current && current !== this.document.body) {
      let selector = current.tagName.toLowerCase();

      if (current.className && typeof current.className === "string") {
        const classes = current.className.trim().split(/\s+/).slice(0, 2);
        if (classes.length > 0 && classes[0]) {
          selector += `.${classes.map(c => CSS.escape(c)).join(".")}`;
        }
      }

      const siblings = current.parentElement?.children || [];
      const sameTagSiblings = Array.from(siblings).filter(s => s.tagName === current.tagName);
      if (sameTagSiblings.length > 1) {
        const index = sameTagSiblings.indexOf(current) + 1;
        selector += `:nth-of-type(${index})`;
      }

      path.unshift(selector);
      current = current.parentElement;
    }

    return path.join(" > ");
  }
}
