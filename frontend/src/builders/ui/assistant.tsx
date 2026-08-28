/**
 * Builders content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module only paints feed content into the slot.
 */

import { renderFeed } from './feed.tsx';

export { renderFeed };

export function createBuildersContentAdapter() {
  return {
    mount(slotEl: HTMLElement, _ctx: { host?: unknown } = {}) {
      void renderFeed(slotEl);
      return {
        unmount() {
          slotEl.replaceChildren();
        },
      };
    },
  };
}
