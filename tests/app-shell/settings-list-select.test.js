// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
  mountSettingsListSelects,
  syncSettingsListSelect,
} from '../../frontend/src/app-shell/ui/settings/list-select.ts';
import { closeFloatingListSelect } from '../../frontend/src/shared/floating-list-select.ts';

function select(id, options) {
  const el = document.createElement('select');
  el.id = id;
  for (const [value, label] of options) {
    const opt = document.createElement('option');
    opt.value = value;
    opt.textContent = label;
    el.appendChild(opt);
  }
  document.body.appendChild(el);
  return el;
}

describe('settings list selects', () => {
  it('mirrors Agent and MCP selects with the Notes menu', () => {
    document.body.innerHTML = '';
    const engine = select('settings-llm-engine', [
      ['host', 'GLM'],
      ['openai', 'OpenAI'],
    ]);
    select('settings-mcp-channel', [['cursor', '/mcp/cursor']]);
    select('settings-mcp-ticket-channel', [['spark', 'Spark']]);
    mountSettingsListSelects();

    const triggers = document.querySelectorAll('.settings-list-select .list-select-trigger');
    expect(triggers).toHaveLength(3);
    expect(triggers[0].textContent).toMatch(/GLM/);
    expect(engine.classList.contains('settings-native-select')).toBe(true);

    triggers[0].click();
    document.querySelector('.settings-select-menu [data-value="openai"]').click();
    expect(engine.value).toBe('openai');
    expect(triggers[0].textContent).toMatch(/OpenAI/);
    closeFloatingListSelect();
  });

  it('follows a programmatic category change', () => {
    document.body.innerHTML = '';
    const engine = select('settings-llm-engine', [
      ['host', 'GLM'],
      ['grok', 'Grok'],
    ]);
    mountSettingsListSelects();
    engine.value = 'grok';
    syncSettingsListSelect(engine);
    expect(document.querySelector('.settings-list-select .list-select-label')?.textContent).toBe('Grok');
  });
});
