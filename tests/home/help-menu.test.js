// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('help menu · source', () => {
  it('puts HelpMenu beside AccountBar in one footer row', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/help-menu\.tsx['"]/);
    expect(page).toMatch(/data-role="account-row"/);
    expect(page).toMatch(/<HelpMenu/);
    const rowStart = page.indexOf('data-role="account-row"');
    const account = page.indexOf('<AccountBar', rowStart);
    const help = page.indexOf('<HelpMenu', rowStart);
    expect(account).toBeGreaterThan(rowStart);
    expect(help).toBeGreaterThan(account);

    const bar = readRel('frontend/src/home/ui/account-bar.tsx');
    expect(bar).not.toMatch(/HelpMenu|help-menu/);
  });

  it('defines HelpMenu with a gear and Settings, Devices, About', () => {
    const path = join(repoRoot, 'frontend/src/home/ui/help-menu.tsx');
    expect(existsSync(path)).toBe(true);
    const src = readRel('frontend/src/home/ui/help-menu.tsx');
    expect(src).toMatch(/export function HelpMenu/);
    expect(src).toMatch(/aria-label="Settings"/);
    expect(src).toMatch(/<HomeNavIcon name="settings"/);
    expect(src).not.toMatch(/>\?<\/span>/);
    const settings = src.indexOf('data-role="help-menu-settings"');
    const devices = src.indexOf('data-role="help-menu-devices"');
    const about = src.indexOf('data-role="help-menu-about"');
    expect(settings).toBeGreaterThan(-1);
    expect(devices).toBeGreaterThan(settings);
    expect(about).toBeGreaterThan(devices);
    expect(src).not.toMatch(/Keyboard shortcuts|Sign in/);
    expect(src).not.toMatch(/getPackageSnapshot|ApiInvokeMap/);
  });

  it('styles a circular help trigger next to the account bar', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-row\b/);
    expect(css).toMatch(/\.home-help-menu-trigger\b/);
    expect(css).toMatch(/\.home-help-menu-trigger\s*\{[^}]*border-radius:\s*50%/);
  });
});

describe('HelpMenu', () => {
  let container;
  let root;
  let HelpMenu;

  beforeEach(async () => {
    ({ HelpMenu } = await import('../../frontend/src/home/ui/help-menu.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(props) {
    act(() => {
      root.render(createElement(HelpMenu, props));
    });
  }

  function trigger() {
    return container.querySelector('[data-role="help-menu-trigger"]');
  }

  it('shows Settings, Devices, About after the gear is clicked', () => {
    render();
    expect(container.querySelector('[data-role="help-menu"]')).not.toBeNull();
    expect(trigger()?.getAttribute('aria-label')).toBe('Settings');
    expect(container.querySelector('[data-home-icon="settings"]')).not.toBeNull();
    expect(container.querySelector('[data-role="help-menu-about"]')).toBeNull();

    act(() => {
      trigger().click();
    });
    const items = container.querySelectorAll('[role="menuitem"]');
    expect(items.length).toBe(3);
    expect([...items].map((item) => item.getAttribute('data-role'))).toEqual([
      'help-menu-settings',
      'help-menu-devices',
      'help-menu-about',
    ]);
    expect([...items].map((item) => item.textContent)).toEqual(['Settings', 'Devices', 'About']);
    expect(items[0].id).toBe('btn-settings');
    expect(items[1].id).toBe('btn-bind');
  });

  it('calls onSettings, onDevices, onAbout and closes the menu', () => {
    const onSettings = vi.fn();
    const onDevices = vi.fn();
    const onAbout = vi.fn();
    render({ onSettings, onDevices, onAbout });
    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="help-menu-settings"]').click();
    });
    expect(onSettings).toHaveBeenCalledTimes(1);
    expect(container.querySelector('[data-role="help-menu-settings"]')).toBeNull();

    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="help-menu-devices"]').click();
    });
    expect(onDevices).toHaveBeenCalledTimes(1);

    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="help-menu-about"]').click();
    });
    expect(onAbout).toHaveBeenCalledTimes(1);
    expect(container.querySelector('[data-role="help-menu-about"]')).toBeNull();
  });

  it('stays usable when onAbout throws', () => {
    const onAbout = vi.fn(() => {
      throw new Error('about failed');
    });
    render({ onAbout });
    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="help-menu-about"]').click();
    });
    expect(container.querySelector('[data-role="help-menu"]')).not.toBeNull();
    act(() => {
      trigger().click();
    });
    expect(container.querySelector('[data-role="help-menu-about"]')).not.toBeNull();
  });
});
