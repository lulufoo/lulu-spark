// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { positionFloatingListMenu } from './floating-list-menu.ts';

/**
 * @typedef {{ value: string, label: string, title?: string }} FloatingListSelectOption
 */

/** @type {{ menu: HTMLElement, trigger: HTMLElement, picker: HTMLElement, docListener: (event: MouseEvent) => void } | null} */
let _openState = null;

export function closeFloatingListSelect() {
  if (!_openState) return;
  const { menu, trigger, picker, docListener } = _openState;
  trigger.setAttribute('aria-expanded', 'false');
  picker.classList.remove('is-open');
  menu.remove();
  document.removeEventListener('mousedown', docListener);
  _openState = null;
}

/**
 * @param {{
 *   ariaLabel: string,
 *   value: string,
 *   options: FloatingListSelectOption[],
 *   pickerClass?: string,
 *   onSelect: (value: string) => void,
 * }} config
 * @returns {{ picker: HTMLElement, sync: (next: { value: string, options: FloatingListSelectOption[] }) => void }}
 */
export function createFloatingListSelect({ ariaLabel, value, options, pickerClass = '', onSelect }) {
  const picker = document.createElement('div');
  picker.className = ['list-select-picker', pickerClass].filter(Boolean).join(' ');

  const trigger = document.createElement('button');
  trigger.type = 'button';
  trigger.className = 'list-select-trigger';
  trigger.setAttribute('aria-label', ariaLabel);
  trigger.setAttribute('aria-haspopup', 'listbox');
  trigger.setAttribute('aria-expanded', 'false');

  const labelSpan = document.createElement('span');
  labelSpan.className = 'list-select-label';

  const chevron = document.createElement('span');
  chevron.className = 'list-select-chevron';
  chevron.setAttribute('aria-hidden', 'true');

  trigger.append(labelSpan, chevron);
  picker.appendChild(trigger);

  /** @type {string} */
  let currentValue = value;
  /** @type {FloatingListSelectOption[]} */
  let currentOptions = options;

  function findLabel(val) {
    return currentOptions.find((o) => o.value === val)?.label ?? currentOptions[0]?.label ?? '';
  }

  function syncTriggerLabel() {
    labelSpan.textContent = findLabel(currentValue);
  }

  function publishTestState() {
    picker._listSelectOptions = currentOptions;
    picker._listSelectValue = currentValue;
  }

  syncTriggerLabel();
  publishTestState();

  function openMenu() {
    if (_openState?.picker === picker) {
      closeFloatingListSelect();
      return;
    }
    closeFloatingListSelect();
    if (!currentOptions.length) return;

    const rect = trigger.getBoundingClientRect();
    const menu = document.createElement('div');
    menu.className = 'list-select-menu';
    menu.setAttribute('role', 'listbox');

    for (const opt of currentOptions) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'list-select-option' + (opt.value === currentValue ? ' selected' : '');
      btn.dataset.value = opt.value;
      btn.textContent = opt.label;
      if (opt.title) btn.title = opt.title;
      menu.appendChild(btn);
    }

    document.body.appendChild(menu);

    const docListener = (event) => {
      if (event.target.closest('.list-select-trigger') || event.target.closest('.list-select-menu')) {
        return;
      }
      closeFloatingListSelect();
    };

    _openState = { menu, trigger, picker, docListener };

    const clipped = positionFloatingListMenu(menu, rect);
    trigger.setAttribute('aria-expanded', 'true');
    picker.classList.add('is-open');
    document.addEventListener('mousedown', docListener);

    menu.addEventListener('click', (event) => {
      const option = event.target.closest('.list-select-option');
      if (!option) return;
      const next = option.dataset.value ?? '';
      closeFloatingListSelect();
      if (next === currentValue) return;
      currentValue = next;
      publishTestState();
      syncTriggerLabel();
      onSelect(next);
    });

    if (clipped) {
      menu.querySelector('.list-select-option.selected')?.scrollIntoView?.({ block: 'nearest' });
    }
  }

  trigger.addEventListener('click', openMenu);

  return {
    picker,
    sync(next) {
      currentValue = next.value;
      currentOptions = next.options;
      publishTestState();
      syncTriggerLabel();
    },
  };
}
