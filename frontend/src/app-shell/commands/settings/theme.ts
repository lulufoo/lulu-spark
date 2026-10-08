import * as api from '../../../host/api.ts';
import { applyTheme, parseThemeId, preferredThemeId } from '../../../theme.ts';
import { syncSettingsListSelect } from '../../ui/settings/list-select.ts';

export function loadThemePanel(theme?: string) {
  const select = document.getElementById('settings-theme');
  if (!(select instanceof HTMLSelectElement)) return;
  select.value = parseThemeId(theme) ?? preferredThemeId();
  syncSettingsListSelect(select);
}

export async function saveThemeSelection(value: string) {
  const id = parseThemeId(value);
  if (!id) return;
  applyTheme(id);
  await api.setConfig({ theme: id });
}
