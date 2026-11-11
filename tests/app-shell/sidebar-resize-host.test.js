// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
  readSidebarLayoutStartWidth,
  sidebarResizeHost,
} from '../../frontend/src/app-shell/ui/sidebar-resize-host.ts';

describe('sidebar resize host', () => {
  it('uses the layout root when the aside is inside SidebarLayout', () => {
    const layout = document.createElement('div');
    layout.className = 'sidebar-layout';
    const aside = document.createElement('aside');
    layout.appendChild(aside);
    document.body.append(layout);
    expect(sidebarResizeHost(aside)).toBe(layout);
    layout.remove();
  });

  it('falls back to the aside when there is no layout', () => {
    const aside = document.createElement('aside');
    document.body.append(aside);
    expect(sidebarResizeHost(aside)).toBe(aside);
    aside.remove();
  });

  it('reads --sidebar-layout-width before the aside box', () => {
    const layout = document.createElement('div');
    layout.className = 'sidebar-layout';
    layout.style.setProperty('--sidebar-layout-width', '221px');
    const aside = document.createElement('aside');
    aside.style.width = '180px';
    aside.getBoundingClientRect = () => ({ width: 160 });
    layout.appendChild(aside);
    document.body.append(layout);
    expect(readSidebarLayoutStartWidth(aside)).toBe(221);
    layout.remove();
  });
});
