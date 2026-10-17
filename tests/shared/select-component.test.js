// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Select } from '../../frontend/src/shared/components/select/index.ts';

let root;
let host;

function mount(props) {
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  const render = (next) => act(() => root.render(createElement(Select, { ariaLabel: 'Pick', ...next })));
  render(props);
  return render;
}

const trigger = () => host.querySelector('.list-select-trigger');
const menu = () => document.querySelector('.ui-select-menu');
const labels = () => [...document.querySelectorAll('.ui-select-menu .list-select-option')].map((b) => b.textContent);
const option = (value) => document.querySelector(`.list-select-option[data-value="${value}"]`);

afterEach(() => {
  if (root) act(() => root.unmount());
  root = null;
  document.body.innerHTML = '';
});

const OPTIONS = [
  { value: 'a', label: 'Alpha' },
  { value: 'b', label: 'Beta' },
];

describe('Select', () => {
  it('renders the shared picker with the selected label and the aria label', () => {
    mount({ value: 'b', options: OPTIONS, onChange: () => {} });
    expect(host.querySelector('.ui-select-host .ui-select.list-select-picker')).not.toBeNull();
    expect(host.querySelector('.list-select-label').textContent).toBe('Beta');
    expect(trigger().getAttribute('aria-label')).toBe('Pick');
  });

  it('opens a menu with every option, marks the selected one, and closes on an outside click', () => {
    mount({ value: 'a', options: OPTIONS, onChange: () => {} });
    expect(menu()).toBeNull();
    act(() => trigger().click());
    expect(labels()).toEqual(['Alpha', 'Beta']);
    expect(menu().querySelector('.list-select-option.selected').textContent).toBe('Alpha');
    expect(trigger().getAttribute('aria-expanded')).toBe('true');
    act(() => document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })));
    expect(menu()).toBeNull();
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
  });

  it('reports a chosen option once, closes, and does not report the current value again', () => {
    const onChange = vi.fn();
    mount({ value: 'a', options: OPTIONS, onChange });
    act(() => trigger().click());
    act(() => option('a').click());
    expect(onChange).not.toHaveBeenCalled();
    act(() => trigger().click());
    act(() => option('b').click());
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('b');
    expect(menu()).toBeNull();
  });

  it('follows new props without remounting the trigger', () => {
    const render = mount({ value: 'a', options: OPTIONS, onChange: () => {} });
    const before = trigger();
    render({ value: 'c', options: [...OPTIONS, { value: 'c', label: 'Gamma' }], onChange: () => {} });
    expect(trigger()).toBe(before);
    expect(host.querySelector('.list-select-label').textContent).toBe('Gamma');
  });

  it('calls the latest onChange', () => {
    const first = vi.fn();
    const second = vi.fn();
    const render = mount({ value: 'a', options: OPTIONS, onChange: first });
    render({ value: 'a', options: OPTIONS, onChange: second });
    act(() => trigger().click());
    act(() => option('b').click());
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith('b');
  });

  it('does not open a menu when there are no options', () => {
    mount({ value: '', options: [], onChange: () => {} });
    act(() => trigger().click());
    expect(menu()).toBeNull();
    expect(host.querySelector('.list-select-label').textContent).toBe('');
  });

  it('shows the first label for a value that is not an option, and removes an open menu on unmount', () => {
    mount({ value: 'zzz', options: OPTIONS, onChange: () => {} });
    expect(host.querySelector('.list-select-label').textContent).toBe('Alpha');
    act(() => trigger().click());
    expect(menu()).not.toBeNull();
    act(() => root.unmount());
    root = null;
    expect(menu()).toBeNull();
  });
});
