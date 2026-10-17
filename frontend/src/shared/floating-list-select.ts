import { positionFloatingListMenu } from './floating-list-menu.ts';
import type {
  FloatingListOpenState,
  FloatingListSelectConfig,
  FloatingListSelectOption,
  FloatingListSelectSync,
  ListSelectPicker,
} from './types.ts';

let _openState: FloatingListOpenState | null = null;

export function closeFloatingListSelect() {
  if (!_openState) return;
  const { menu, trigger, picker, docListener } = _openState;
  trigger.setAttribute('aria-expanded', 'false');
  picker.classList.remove('is-open');
  menu.remove();
  document.removeEventListener('mousedown', docListener);
  _openState = null;
}

function showMenu({
  picker,
  trigger,
  pickerClass,
  value,
  options,
  menuMaxHeight,
  onSelect,
}: {
  picker: HTMLElement;
  trigger: HTMLElement;
  pickerClass: string;
  value: string;
  options: FloatingListSelectOption[];
  menuMaxHeight?: number;
  onSelect: (next: string) => void;
}) {
  if (_openState?.picker === picker) {
    closeFloatingListSelect();
    return;
  }
  closeFloatingListSelect();
  if (!options.length) return;

  const rect = trigger.getBoundingClientRect();
  const menu = document.createElement('div');
  menu.className = ['list-select-menu', pickerClass ? `${pickerClass}-menu` : '']
    .filter(Boolean)
    .join(' ');
  menu.setAttribute('role', 'listbox');

  for (const opt of options) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'list-select-option' + (opt.value === value ? ' selected' : '');
    btn.dataset.value = opt.value;
    btn.textContent = opt.label;
    if (opt.title) btn.title = opt.title;
    menu.appendChild(btn);
  }

  document.body.appendChild(menu);

  const docListener = (event: MouseEvent) => {
    const target = event.target as Node | null;
    if (!target) return;
    if (trigger.contains(target) || menu.contains(target)) return;
    closeFloatingListSelect();
  };

  _openState = { menu, trigger, picker, docListener };

  const clipped = positionFloatingListMenu(menu, rect, window.innerHeight, menuMaxHeight);
  trigger.setAttribute('aria-expanded', 'true');
  picker.classList.add('is-open');
  document.addEventListener('mousedown', docListener);

  menu.addEventListener('click', (event) => {
    const target = event.target as Element | null;
    const option = target?.closest('.list-select-option') as HTMLElement | null;
    if (!option) return;
    const next = option.dataset.value ?? '';
    closeFloatingListSelect();
    if (next === value) return;
    onSelect(next);
  });

  if (clipped) {
    menu.querySelector('.list-select-option.selected')?.scrollIntoView?.({ block: 'nearest' });
  }
}

/** Open the shared list-select menu from an existing control (a badge, a header button). */
export function openAnchoredListSelect({
  trigger,
  options,
  value,
  onSelect,
  pickerClass = 'ui-select',
  menuMaxHeight,
}: {
  trigger: HTMLElement;
  options: FloatingListSelectOption[];
  value: string;
  onSelect: (next: string) => void;
  pickerClass?: string;
  menuMaxHeight?: number;
}) {
  showMenu({
    picker: trigger,
    trigger,
    pickerClass,
    value,
    options,
    menuMaxHeight,
    onSelect,
  });
}

export function createFloatingListSelect({
  ariaLabel,
  value,
  options,
  pickerClass = '',
  menuMaxHeight,
  onSelect,
}: FloatingListSelectConfig): { picker: ListSelectPicker; sync: (next: FloatingListSelectSync) => void } {
  const picker = document.createElement('div') as ListSelectPicker;
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
  chevron.innerHTML =
    '<svg class="list-select-chevron-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>';

  trigger.append(labelSpan, chevron);
  picker.appendChild(trigger);

  let currentValue = value;
  let currentOptions: FloatingListSelectOption[] = options;

  function findLabel(val: string) {
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
    showMenu({
      picker,
      trigger,
      pickerClass,
      value: currentValue,
      options: currentOptions,
      menuMaxHeight,
      onSelect: (next) => {
        currentValue = next;
        publishTestState();
        syncTriggerLabel();
        onSelect(next);
      },
    });
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
