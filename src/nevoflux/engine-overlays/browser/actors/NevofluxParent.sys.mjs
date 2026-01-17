/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

export class NevofluxParent extends JSWindowActorParent {
  // Track pending dialog for this browsing context
  static _pendingDialogs = new WeakMap();

  constructor() {
    super();
    this._dialogObserver = null;
  }

  actorCreated() {
    // Register dialog observer when actor is created
    this._setupDialogObserver();
  }

  didDestroy() {
    // Cleanup observer when actor is destroyed
    this._removeDialogObserver();
  }

  _setupDialogObserver() {
    if (this._dialogObserver) return;

    this._dialogObserver = {
      observe: (subject, topic, data) => {
        if (topic === "common-dialog-loaded") {
          // Store dialog reference for this window
          const dominated = subject.opener;
          if (dominated) {
            NevofluxParent._pendingDialogs.set(dominated, subject);
          }
        }
      }
    };

    try {
      Services.obs.addObserver(this._dialogObserver, "common-dialog-loaded");
    } catch (e) {
      // Observer already added or Services not available
    }
  }

  _removeDialogObserver() {
    if (this._dialogObserver) {
      try {
        Services.obs.removeObserver(this._dialogObserver, "common-dialog-loaded");
      } catch (e) {
        // Observer already removed
      }
      this._dialogObserver = null;
    }
  }

  receiveMessage({ name, data }) {
    switch (name) {
      case "dialogAccept":
        return this.acceptDialog(data?.text);
      case "dialogDismiss":
        return this.dismissDialog();
      default:
        return null;
    }
  }

  acceptDialog(text) {
    try {
      const win = this.browsingContext.topChromeWindow;
      const dialog = NevofluxParent._pendingDialogs.get(win);

      if (!dialog) {
        // No dialog present - silently succeed
        return { success: true };
      }

      // Handle prompt input
      if (text !== undefined && dialog.ui?.loginTextbox) {
        dialog.ui.loginTextbox.value = text;
      }

      // Click accept button
      if (dialog.ui?.button0) {
        dialog.ui.button0.click();
      }

      NevofluxParent._pendingDialogs.delete(win);
      return { success: true };
    } catch (e) {
      return { success: false, error: { code: 11001, message: String(e), recoverable: false } };
    }
  }

  dismissDialog() {
    try {
      const win = this.browsingContext.topChromeWindow;
      const dialog = NevofluxParent._pendingDialogs.get(win);

      if (!dialog) {
        // No dialog present - silently succeed
        return { success: true };
      }

      // Click cancel button (button1) if exists, otherwise accept
      if (dialog.ui?.button1) {
        dialog.ui.button1.click();
      } else if (dialog.ui?.button0) {
        dialog.ui.button0.click();
      }

      NevofluxParent._pendingDialogs.delete(win);
      return { success: true };
    } catch (e) {
      return { success: false, error: { code: 11002, message: String(e), recoverable: false } };
    }
  }
}
