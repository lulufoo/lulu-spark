// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const getPackageSnapshotMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/host/api.ts', () => ({
  getPackageSnapshot: (...args) => getPackageSnapshotMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const snapshot = {
  is_debug: true,
  version: 'Beta v0.1',
  product_name: 'Lulu Spark',
};

describe('about · source', () => {
  it('Home page opens About through commands and paints the dialog', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/about-dialog\.tsx['"]/);
    expect(page).toMatch(/<AboutDialog/);
    expect(page).toMatch(/onAbout=\{/);
    expect(page).toMatch(/openAbout/);
    expect(page).not.toMatch(/getPackageSnapshot/);
    expect(page).not.toMatch(/ApiInvokeMap/);
  });

  it('command reads host/api and does not paint', () => {
    const path = join(repoRoot, 'frontend/src/home/commands/about.ts');
    expect(existsSync(path)).toBe(true);
    const src = readRel('frontend/src/home/commands/about.ts');
    expect(src).toMatch(/getPackageSnapshot/);
    expect(src).not.toMatch(/className|AboutDialog/);
  });

  it('dialog paints product name and version only', () => {
    const src = readRel('frontend/src/home/ui/about-dialog.tsx');
    expect(src).toMatch(/about-product-name/);
    expect(src).toMatch(/about-version/);
    expect(src).not.toMatch(/is_debug|isDebug/);
    expect(src).not.toMatch(/getPackageSnapshot|ApiInvokeMap/);
  });

  it('paints About header dismiss as OverlayDismissButton with no Close copy', () => {
    const src = readRel('frontend/src/home/ui/about-dialog.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-about-close["']/);
    expect(src).not.toMatch(/about-dialog-actions/);
    expect(src).not.toMatch(/>\s*Close\s*</);
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/#about-dialog-header\s*\{/);
    expect(css).not.toMatch(/#about-dialog-actions/);
  });
});

describe('openAbout', () => {
  beforeEach(async () => {
    getPackageSnapshotMock.mockReset();
    const { aboutStore } = await import('../../frontend/src/home/state/about.ts');
    aboutStore.set({ open: false, product_name: '', version: '' });
  });

  it('stores product_name and version from the snapshot', async () => {
    getPackageSnapshotMock.mockResolvedValue(snapshot);
    const { openAbout } = await import('../../frontend/src/home/commands/about.ts');
    const { aboutStore } = await import('../../frontend/src/home/state/about.ts');
    await openAbout();
    expect(aboutStore.getSnapshot()).toEqual({
      open: true,
      product_name: 'Lulu Spark',
      version: 'Beta v0.1',
    });
  });

  it('still opens when invoke fails', async () => {
    getPackageSnapshotMock.mockRejectedValue(new Error('no host'));
    const { openAbout } = await import('../../frontend/src/home/commands/about.ts');
    const { aboutStore } = await import('../../frontend/src/home/state/about.ts');
    await openAbout();
    expect(aboutStore.getSnapshot().open).toBe(true);
    expect(aboutStore.getSnapshot().product_name).toBe('');
    expect(aboutStore.getSnapshot().version).toBe('');
  });
});

describe('About dialog + help menu', () => {
  let container;
  let root;

  beforeEach(async () => {
    getPackageSnapshotMock.mockReset();
    getPackageSnapshotMock.mockResolvedValue(snapshot);
    const { aboutStore } = await import('../../frontend/src/home/state/about.ts');
    aboutStore.set({ open: false, product_name: '', version: '' });
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

  it('About menu item opens the dialog with snapshot copy', async () => {
    const { HelpMenu } = await import('../../frontend/src/home/ui/help-menu.tsx');
    const { AboutDialog } = await import('../../frontend/src/home/ui/about-dialog.tsx');
    const { openAbout } = await import('../../frontend/src/home/commands/about.ts');
    act(() => {
      root.render(
        createElement('div', null, [
          createElement(HelpMenu, { key: 'help', onAbout: () => void openAbout() }),
          createElement(AboutDialog, { key: 'about' }),
        ]),
      );
    });
    expect(container.querySelector('[data-role="about-dialog"]').getAttribute('data-open')).toBe(
      'false',
    );

    act(() => {
      container.querySelector('[data-role="help-menu-trigger"]').click();
    });
    await act(async () => {
      container.querySelector('[data-role="help-menu-about"]').click();
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="about-dialog"]').getAttribute('data-open')).toBe(
        'true',
      );
    });
    expect(container.querySelector('[data-role="about-product-name"]').textContent).toBe(
      'Lulu Spark',
    );
    expect(container.querySelector('[data-role="about-version"]').textContent).toBe('Beta v0.1');

    const closeBtn = container.querySelector('#btn-about-close');
    expect(closeBtn).not.toBeNull();
    expect(closeBtn.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(String(closeBtn.textContent ?? '').trim()).toBe('✕');
    expect(String(closeBtn.textContent ?? '')).not.toContain('Close');
    expect(container.querySelector('#about-dialog-header button')).toBe(closeBtn);

    act(() => {
      closeBtn.click();
    });
    expect(container.querySelector('[data-role="about-dialog"]').getAttribute('data-open')).toBe(
      'false',
    );
  });

  it('keeps About close in commands and not inside the shared control', async () => {
    const dialogSrc = readRel('frontend/src/home/ui/about-dialog.tsx');
    const commandSrc = readRel('frontend/src/home/commands/about.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export function closeAbout/);
    expect(dialogSrc).toMatch(/closeAbout/);
    expect(sharedSrc).not.toMatch(/closeAbout/);
  });
});
