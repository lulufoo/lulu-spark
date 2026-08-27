import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach } from 'vitest';

import { getBaselineEntries } from '../frontend/js/home-entry-shell/entry-config.js';
import { createContentRegistry } from '../frontend/js/home-entry-shell/content-registry.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const BASELINE_KEYS = ['read-later', 'todo-task', 'notes', 'builders'];
const BASELINE_TITLES = {
  'read-later': 'Read Later',
  'todo-task': 'Todos',
  notes: 'Notes Assistant',
  builders: 'Builders',
};

describe('home-entry-shell contract · EntryConfig + ContentRegistry (T1)', () => {
  /** @type {ReturnType<typeof createContentRegistry>} */
  let registry;

  beforeEach(() => {
    registry = createContentRegistry();
  });

  it('baseline four keys: id aligns with contentKey and titles match existing FAB titles', () => {
    const entries = getBaselineEntries();
    expect(entries).toHaveLength(4);

    const byKey = Object.fromEntries(entries.map((e) => [e.contentKey, e]));
    for (const key of BASELINE_KEYS) {
      const entry = byKey[key];
      expect(entry, `missing baseline entry ${key}`).toBeTruthy();
      expect(entry.id).toBe(key);
      expect(entry.contentKey).toBe(key);
      expect(entry.title).toBe(BASELINE_TITLES[key]);
      expect(typeof entry.overlayTitle).toBe('string');
      expect(entry.overlayTitle.length).toBeGreaterThan(0);
      expect(typeof entry.fabClass).toBe('string');
      expect(entry.fabClass.length).toBeGreaterThan(0);
      expect(typeof entry.iconPaths).toBe('string');
      expect(entry.iconPaths.length).toBeGreaterThan(0);
      expect(entry.panelWidth).toBeGreaterThan(0);
      expect(entry.panelHeight).toBeGreaterThan(0);
    }
    expect(byKey.builders.panelWidth).toBe(480);
    expect(byKey.builders.panelHeight).toBe(520);
  });

  it('ContentRegistry registers and retrieves an adapter per baseline contentKey', () => {
    for (const key of BASELINE_KEYS) {
      const adapter = { contentKey: key, mount: () => {} };
      registry.register(key, adapter);
      expect(registry.get(key)).toBe(adapter);
    }
  });

  it('unregistered contentKey yields undefined (no successful lookup)', () => {
    expect(registry.get('ai-assistant')).toBeUndefined();
    expect(registry.get('missing-key')).toBeUndefined();
  });

  it('EntryConfig / ContentRegistry / OverlayChrome concerns stay separable', () => {
    const entries = getBaselineEntries();
    for (const entry of entries) {
      // EntryConfig owns display metadata OverlayChrome would consume — no adapter payload.
      expect(entry).not.toHaveProperty('adapter');
      expect(entry).not.toHaveProperty('mount');
      expect(entry.contentKey).toBeTruthy();
      expect(entry.overlayTitle).toBeTruthy();
    }

    // ContentRegistry maps contentKey → adapter only; it does not invent chrome fields.
    const adapter = { mount: () => {} };
    registry.register('read-later', adapter);
    const got = registry.get('read-later');
    expect(got).toBe(adapter);
    expect(got).not.toHaveProperty('overlayTitle');
    expect(got).not.toHaveProperty('title');
  });

  it('adding or removing an entry only touches config + registry modules (not shell/fsm)', () => {
    const entryConfigSrc = readFileSync(
      join(repoRoot, 'frontend/js/home-entry-shell/entry-config.js'),
      'utf8',
    );
    const registrySrc = readFileSync(
      join(repoRoot, 'frontend/js/home-entry-shell/content-registry.js'),
      'utf8',
    );

    for (const src of [entryConfigSrc, registrySrc]) {
      expect(src).not.toMatch(/from\s+['"].*\/(shell|fsm)(\.js)?['"]/);
      expect(src).not.toMatch(/mountHomeEntryShell|createHomeEntryFsm/);
    }

    // Config is data-driven: baseline list is an array we can extend without shell edits.
    const entries = getBaselineEntries();
    expect(Array.isArray(entries)).toBe(true);
    expect(entries.every((e) => typeof e.contentKey === 'string')).toBe(true);
  });
});
