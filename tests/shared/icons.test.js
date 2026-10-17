// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import {
  DEFAULT_PROJECT_ICON_ID,
  ICON_CATEGORIES,
  ICONS,
  getIcon,
  isIconId,
  pickerCategories,
} from '../../frontend/src/shared/icons/index.ts';
import { Icon } from '../../frontend/src/shared/icons/icon.tsx';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('icon catalog', () => {
  it('has the 7 categories and 104 icons copied from lulu-board', () => {
    expect(ICON_CATEGORIES.map((c) => c.name)).toEqual([
      'Shapes',
      'Process',
      'People',
      'Content',
      'Tech',
      'Marks',
      'Directions',
    ]);
    expect(ICON_CATEGORIES.map((c) => c.code)).toEqual(['00', '01', '02', '03', '04', '05', '06']);
    expect(ICONS).toHaveLength(104);
    const count = (code) => ICONS.filter((i) => i.category === code).length;
    expect(ICON_CATEGORIES.map((c) => count(c.code))).toEqual([18, 24, 11, 7, 12, 17, 15]);
  });

  it('gives every icon a well-formed id that starts with its category code', () => {
    for (const icon of ICONS) {
      expect(icon.id, icon.name).toMatch(/^0[0-6][a-z0-9]{4}$/);
      expect(icon.id.slice(0, 2), icon.name).toBe(icon.category);
      expect(isIconId(icon.id)).toBe(true);
    }
  });

  it('has unique ids and non-empty names and drawings', () => {
    expect(new Set(ICONS.map((i) => i.id)).size).toBe(ICONS.length);
    for (const icon of ICONS) {
      expect(icon.name.trim(), icon.id).not.toBe('');
      expect(icon.body.trim(), icon.id).toMatch(/^<(path|circle|rect|line|polyline|polygon|ellipse)\b/);
    }
  });

  it('does not depend on lulu-board at runtime', () => {
    for (const file of ['catalog.ts', 'index.ts', 'icon.tsx']) {
      const source = readFileSync(join(root, 'frontend/src/shared/icons', file), 'utf8');
      expect(source.replace(/\/\*[\s\S]*?\*\//, '')).not.toMatch(/lulu-board/);
      expect(source).not.toMatch(/from '[^']*lulu-board/);
    }
  });
});

describe('icon lookups', () => {
  it('rejects ids that are not shaped like one', () => {
    for (const bad of ['', 'abc', '07abcd', '03ABCD', '03abc', '03abcde', '0xabcd', undefined, null, 3]) {
      expect(isIconId(bad), String(bad)).toBe(false);
    }
    expect(isIconId('03a1b2')).toBe(true);
  });

  it('returns undefined for an unknown id, also a well-formed one', () => {
    expect(getIcon('nope')).toBeUndefined();
    expect(getIcon('')).toBeUndefined();
    expect(isIconId('00zzzz') && !ICONS.some((i) => i.id === '00zzzz')).toBe(true);
    expect(getIcon('00zzzz')).toBeUndefined();
  });

  it('keeps Directions out of the picker but still resolves its icons', () => {
    const picker = pickerCategories();
    expect(picker.map((c) => c.name)).toEqual(['Shapes', 'Process', 'People', 'Content', 'Tech', 'Marks']);
    expect(picker.reduce((sum, c) => sum + c.icons.length, 0)).toBe(89);
    const direction = ICONS.find((i) => i.category === '06');
    expect(picker.flatMap((c) => c.icons).some((i) => i.id === direction.id)).toBe(false);
    expect(getIcon(direction.id)).toBe(direction);
  });

  it('uses Content/Folder as the default project icon', () => {
    const folder = getIcon(DEFAULT_PROJECT_ICON_ID);
    expect(folder.name).toBe('Folder');
    expect(folder.category).toBe('03');
    expect(ICON_CATEGORIES.find((c) => c.code === folder.category).name).toBe('Content');
  });
});

describe('<Icon>', () => {
  function render(props) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const reactRoot = createRoot(host);
    act(() => reactRoot.render(createElement(Icon, props)));
    return { host, unmount: () => act(() => reactRoot.unmount()) };
  }

  it('draws a 24x24 outline in the current colour', () => {
    const { host, unmount } = render({ id: DEFAULT_PROJECT_ICON_ID });
    const svg = host.querySelector('svg');
    expect(svg.getAttribute('viewBox')).toBe('0 0 24 24');
    expect(svg.getAttribute('stroke')).toBe('currentColor');
    expect(svg.getAttribute('fill')).toBe('none');
    expect(svg.getAttribute('stroke-width')).toBe('2');
    expect(svg.getAttribute('width')).toBe('16');
    expect(svg.dataset.iconId).toBe(DEFAULT_PROJECT_ICON_ID);
    expect(svg.querySelector('path, rect, circle')).not.toBeNull();
    unmount();
  });

  it('takes a size and renders nothing for an unknown id', () => {
    const sized = render({ id: DEFAULT_PROJECT_ICON_ID, size: 20 });
    expect(sized.host.querySelector('svg').getAttribute('height')).toBe('20');
    sized.unmount();
    const unknown = render({ id: 'unknown' });
    expect(unknown.host.innerHTML).toBe('');
    unknown.unmount();
  });
});
