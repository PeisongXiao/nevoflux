/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

export class NevofluxParent extends JSWindowActorParent {
  receiveMessage({ name, data }) {
    // Parent can receive messages from child if needed
    return null;
  }
}
