// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  closeFloatingListSelect,
  createFloatingListSelect,
} from '../frontend/js/components/floating-list-select.js';

describe('floating-list-select', () => {
  afterEach(() => {
    closeFloatingListSelect();
    document.body.innerHTML = '';
  });

  it('renders trigger with selected label', () => {
    const host = document.createElement('div');
    document.body.appendChild(host);

    const { picker } = createFloatingListSelect({
      ariaLabel: '筛选主题',
      pickerClass: 'topic-select',
      value: 'ai',
      options: [
        { value: '', label: '全部 (4)' },
        { value: 'ai', label: 'ai (2)' },
      ],
      onSelect: () => {},
    });
    host.appendChild(picker);

    expect(picker.querySelector('.list-select-trigger')).not.toBeNull();
    expect(picker.querySelector('.list-select-label')?.textContent).toBe('ai (2)');
  });

  it('opens menu and calls onSelect', () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    let selected = '';
    const { picker } = createFloatingListSelect({
      ariaLabel: '筛选标签',
      pickerClass: 'tag-select',
      value: '',
      options: [
        { value: '', label: '全部 (4)' },
        { value: 'k1', label: 'Alpha (2)' },
      ],
      onSelect: (v) => { selected = v; },
    });
    host.appendChild(picker);

    picker.querySelector('.list-select-trigger')?.click();
    const menu = document.querySelector('.list-select-menu');
    expect(menu).not.toBeNull();
    menu?.querySelector('[data-value="k1"]')?.click();
    expect(selected).toBe('k1');
    expect(document.querySelector('.list-select-menu')).toBeNull();
  });
});
