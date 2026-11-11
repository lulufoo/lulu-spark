// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { useState } from 'react';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SidebarLayout } from '../../frontend/src/app-shell/ui/sidebar-layout.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appCss = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');

function layoutCss() {
  const rules = [];
  for (const match of appCss.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (match[1].includes('sidebar-layout')) rules.push(`${match[1].trim()} {${match[2]}}`);
  }
  return rules.join('\n');
}

function installLayoutCss() {
  const style = document.createElement('style');
  style.textContent = `:root { --app-header-height: 56px; }\n${layoutCss()}\n#sidebar { display: flex; width: 221px; flex: 0 0 221px; }`;
  document.head.appendChild(style);
  return style;
}

/** Read the grid tracks the browser is using. jsdom does not place grid items, so the box comes from those tracks. */
function trackBox(el, frameWidth) {
  const root = el.closest('.sidebar-layout');
  const sidebarWidth = parseFloat(getComputedStyle(root).getPropertyValue('--sidebar-layout-width'));
  const column = getComputedStyle(el).gridColumn.replace(/\s+/g, '');
  const row = getComputedStyle(el).gridRow.trim();
  if (column === '1/-1') return { row, left: 0, width: frameWidth };
  if (column === '2') return { row, left: sidebarWidth, width: frameWidth - sidebarWidth };
  if (column === '1') return { row, left: 0, width: sidebarWidth };
  throw new Error(`unparsed grid-column "${getComputedStyle(el).gridColumn}"`);
}

function Fixture({ width, frameWidth }) {
  const [collapsed, setCollapsed] = useState(false);
  return createElement(
    'div',
    { 'data-fixture-frame': '', style: { width: frameWidth } },
    createElement('button', {
      type: 'button',
      'data-collapse': '',
      onClick: () => setCollapsed((value) => !value),
    }, 'Collapse'),
    createElement(SidebarLayout, {
      collapsed,
      width,
      navId: 'fixture-nav',
      onToggle: () => setCollapsed((value) => !value),
      tool: createElement('button', { type: 'button', 'data-sidebar-toggle': '' }, 'Sidebar'),
      header: createElement('h1', { 'data-fixture-header': '' }, 'Title'),
      nav: createElement(
        'aside',
        { id: 'sidebar', 'data-fixture-nav-body': '', style: { width: '400px', flexBasis: '400px' } },
        createElement('button', { type: 'button', 'data-fixture-nav': '' }, 'Nav'),
        createElement('div', {
          className: 'sidebar-resizer',
          role: 'separator',
          tabIndex: 0,
          'data-fixture-resizer': '',
        }),
      ),
      children: createElement(
        'article',
        { 'data-fixture-content': '' },
        createElement('button', { type: 'button', 'data-fixture-content-btn': '' }, 'Body'),
      ),
    }),
  );
}

describe('SidebarLayout', () => {
  let container;
  let root;
  let style;
  let originalStorage;

  beforeEach(() => {
    globalThis.IS_REACT_ACT_ENVIRONMENT = true;
    originalStorage = globalThis.localStorage;
    const setItem = vi.fn();
    globalThis.localStorage = {
      getItem: vi.fn(() => null),
      setItem,
      removeItem: vi.fn(),
    };
    style = installLayoutCss();
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    style?.remove();
    document.body.innerHTML = '';
    globalThis.localStorage = originalStorage;
  });

  function render(width = '280px', frameWidth = '1000px') {
    act(() => {
      root.render(createElement(Fixture, { width, frameWidth }));
    });
    return {
      frameWidth: parseFloat(frameWidth),
      layout: container.querySelector('.sidebar-layout'),
      tool: container.querySelector('.sidebar-layout-tool'),
      header: container.querySelector('.sidebar-layout-header'),
      nav: container.querySelector('.sidebar-layout-nav'),
      content: container.querySelector('.sidebar-layout-content'),
    };
  }

  function collapse() {
    act(() => {
      container.querySelector('[data-collapse]').click();
    });
  }

  it('moves the content to the left edge when collapsed and restores the same width', () => {
    const view = render();
    const openContent = trackBox(view.content, view.frameWidth);
    const openHeader = trackBox(view.header, view.frameWidth);
    const openTool = trackBox(view.tool, view.frameWidth);

    expect(openContent).toEqual({ row: '2', left: 280, width: 720 });
    expect(openHeader).toEqual({ row: '1', left: 280, width: 720 });
    expect(openTool).toEqual({ row: '1', left: 0, width: 280 });
    expect(view.layout.style.getPropertyValue('--sidebar-layout-width')).toBe('280px');

    collapse();

    expect(trackBox(view.content, view.frameWidth)).toEqual({ row: '2', left: 0, width: 1000 });
    expect(trackBox(view.header, view.frameWidth)).toEqual({ row: '1', left: 0, width: 1000 });
    expect(view.tool.hasAttribute('hidden')).toBe(true);
    expect(getComputedStyle(view.tool).display).toBe('none');
    expect(getComputedStyle(view.header).paddingLeft).toBe('84px');
    expect(getComputedStyle(view.header).alignItems).toBe('center');
    const restore = container.querySelector('.sidebar-header-restore');
    const title = container.querySelector('[data-fixture-header]');
    expect(restore.hasAttribute('hidden')).toBe(false);
    expect(restore.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(view.layout.style.getPropertyValue('--sidebar-layout-width')).toBe('280px');

    collapse();

    expect(trackBox(view.content, view.frameWidth)).toEqual(openContent);
    expect(view.layout.style.getPropertyValue('--sidebar-layout-width')).toBe('280px');
  });

  it('keeps one nav and one content node, hides the nav, and scrolls inside the slots', () => {
    const view = render();
    const navBody = container.querySelector('[data-fixture-nav-body]');
    const contentBody = container.querySelector('[data-fixture-content]');
    expect(view.nav.hidden).toBe(false);
    expect(view.nav.hasAttribute('inert')).toBe(false);
    view.nav.style.display = 'flex';

    collapse();

    expect(container.querySelector('[data-fixture-nav-body]')).toBe(navBody);
    expect(container.querySelector('[data-fixture-content]')).toBe(contentBody);
    expect(view.nav.hidden).toBe(true);
    expect(view.nav.hasAttribute('inert')).toBe(true);
    expect(getComputedStyle(view.nav).display).toBe('none');
    expect(getComputedStyle(view.content).overflow).toBe('auto');
    expect(getComputedStyle(view.nav).minWidth).toBe('0px');
    expect(globalThis.localStorage.setItem).not.toHaveBeenCalled();
  });

  it('returns focus to the toggle and ignores the narrowest saved width of the old sidebar', () => {
    const view = render('180px', '900px');
    const navButton = container.querySelector('[data-fixture-nav]');
    const contentButton = container.querySelector('[data-fixture-content-btn]');
    act(() => {
      navButton.focus();
    });

    collapse();

    expect(document.activeElement).toBe(
      container.querySelector('.sidebar-header-restore [data-sidebar-toggle]'),
    );
    expect(view.nav.hidden).toBe(true);
    expect(view.layout.style.getPropertyValue('--sidebar-layout-width')).toBe('180px');
    expect(getComputedStyle(view.layout).gridTemplateColumns).not.toContain('400');
    expect(getComputedStyle(view.layout).gridTemplateColumns).not.toContain('221');
    expect(getComputedStyle(container.querySelector('#sidebar')).width).toBe('100%');

    collapse();
    act(() => {
      contentButton.focus();
    });
    collapse();

    expect(document.activeElement).toBe(contentButton);
  });

  it('places the resize handle across both rows of the shared layout', () => {
    const css = layoutCss();
    expect(css).toMatch(/\.sidebar-layout \{[\s\S]*?position:\s*relative;/);
    expect(css).toMatch(/\.sidebar-layout > \.sidebar-resizer \{[\s\S]*?grid-row:\s*1 \/ -1;/);
    expect(css).toMatch(/\.sidebar-layout > \.sidebar-resizer \{[\s\S]*?justify-self:\s*end;/);
    expect(css).toMatch(/\.sidebar-layout > \.sidebar-resizer \{[\s\S]*?margin-right:\s*-6px;/);
    expect(css).toMatch(/\.sidebar-layout\[data-collapsed="true"\] > \.sidebar-resizer \{[\s\S]*?display:\s*none;/);
  });

  it('uses a 56px top row, no motion, and does not store or import commands', () => {
    const css = layoutCss();
    expect(appCss).toMatch(/--app-header-height:\s*56px/);
    expect(css).toMatch(/grid-template-rows:\s*var\(--app-header-height\)\s+minmax\(0,\s*1fr\)/);
    expect(css).toMatch(/grid-template-columns:\s*var\(--sidebar-layout-width\)\s+minmax\(0,\s*1fr\)/);
    expect(css).not.toMatch(/transition|animation|--sidebar-width|--home-chat-sidebar-width/);
    const source = readFileSync(
      join(repoRoot, 'frontend/src/app-shell/ui/sidebar-layout.tsx'),
      'utf8',
    );
    expect(source).not.toMatch(/localStorage|app-shell\/commands|app-shell\/state/);
  });
});
