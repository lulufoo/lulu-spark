/**
 * T7 integration harness — mounts a baseline home-entry shell with stub adapters.
 * Consumed by tests/home-entry-shell/integration.test.js.
 */
import { getBaselineEntries } from '../../frontend/src/home-entry-shell/entry-config.ts';
import { createContentRegistry } from '../../frontend/src/home-entry-shell/content-registry.ts';
import { mountHomeEntryShell } from '../../frontend/src/home-entry-shell/shell.tsx';

const THROW_CLOSE_ENTRY = {
  id: 'throw-close',
  contentKey: 'throw-close-content',
  title: 'Throw Close',
  overlayTitle: 'Throw Close Overlay',
};

function stubAdapter() {
  return {
    mount(slot, ctx) {
      const marker = document.createElement('div');
      marker.className = 'shell-content-marker';
      marker.dataset.entryId = ctx.entry.id;
      marker.textContent = `content:${ctx.entry.id}`;
      slot.appendChild(marker);
      return {
        unmount() {
          marker.remove();
        },
      };
    },
  };
}

function throwOnUnmountAdapter() {
  return {
    mount(slot, ctx) {
      const marker = document.createElement('div');
      marker.className = 'shell-content-marker';
      marker.dataset.entryId = ctx.entry.id;
      slot.appendChild(marker);
      return {
        unmount() {
          throw new Error('close failed');
        },
      };
    },
  };
}

/**
 * @param {{ closeUnmountThrows?: boolean }} [opts]
 */
export function mountShellIntegrationFixture(opts = {}) {
  const { closeUnmountThrows = false } = opts;
  const config = closeUnmountThrows
    ? [...getBaselineEntries(), THROW_CLOSE_ENTRY]
    : [...getBaselineEntries()];

  const registry = createContentRegistry();
  for (const entry of config) {
    if (entry.id === THROW_CLOSE_ENTRY.id) {
      registry.register(entry.contentKey, throwOnUnmountAdapter());
    } else {
      registry.register(entry.contentKey, stubAdapter());
    }
  }

  const anchor = document.createElement('div');
  document.body.appendChild(anchor);
  const shell = mountHomeEntryShell(anchor, { config, registry, host: {} });
  return { anchor, shell, config, registry };
}

export function hub(root) {
  return root.querySelector('[data-role="hub"]');
}

export function entryBtn(root, entryId) {
  return root.querySelector(`[data-entry-id="${entryId}"]`);
}

export function closeBtn(root) {
  return root.querySelector('[data-role="close"]');
}

export function backdrop(root) {
  return root.querySelector('[data-role="backdrop"]');
}

export function overlay(root) {
  return root.querySelector('[data-role="overlay"]');
}

export function contentSlot(root) {
  return root.querySelector('[data-role="content-slot"]');
}
