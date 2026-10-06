import { createFloatingListSelect } from '../../../shared/floating-list-select.ts';
import type { FloatingListSelectSync } from '../../../shared/types.ts';

const SELECT_IDS = ['settings-llm-engine', 'settings-mcp-channel', 'settings-mcp-ticket-channel'];

type Mirror = {
  sync: (next: FloatingListSelectSync) => void;
};

const mirrors = new WeakMap<HTMLSelectElement, Mirror>();

function readOptions(select: HTMLSelectElement) {
  return Array.from(select.options).map((opt) => ({
    value: opt.value,
    label: opt.textContent || opt.value,
  }));
}

export function syncSettingsListSelect(select: HTMLSelectElement | null) {
  if (!select) return;
  mirrors.get(select)?.sync({ value: select.value, options: readOptions(select) });
}

function mountOne(select: HTMLSelectElement) {
  if (mirrors.has(select)) {
    syncSettingsListSelect(select);
    return;
  }
  select.classList.add('settings-native-select');
  const host = document.createElement('div');
  host.className = 'settings-list-select';
  const { picker, sync } = createFloatingListSelect({
    ariaLabel: select.getAttribute('aria-label') || select.id,
    value: select.value,
    options: readOptions(select),
    pickerClass: 'settings-select',
    onSelect: (next) => {
      if (select.value === next) return;
      select.value = next;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    },
  });
  host.appendChild(picker);
  select.after(host);
  mirrors.set(select, { sync });
  select.addEventListener('change', () => syncSettingsListSelect(select));
  const observer = new MutationObserver(() => syncSettingsListSelect(select));
  observer.observe(select, { childList: true });
}

export function mountSettingsListSelects() {
  for (const id of SELECT_IDS) {
    const select = document.getElementById(id);
    if (select instanceof HTMLSelectElement) mountOne(select);
  }
}
