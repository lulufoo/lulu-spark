/**
 * Builders content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module only paints feed content into the slot.
 */

import { renderFeed } from './feed.js';

/**
 * @returns {{ mount: (slotEl: HTMLElement, ctx?: { host?: unknown }) => { unmount: () => void } }}
 */
export function createBuildersContentAdapter() {
  return {
    /**
     * @param {HTMLElement} slotEl
     * @param {{ host?: unknown }} [_ctx]
     */
    mount(slotEl, _ctx = {}) {
      void renderFeed(slotEl);
      return {
        unmount() {
          slotEl.innerHTML = '';
        },
      };
    },
  };
}
