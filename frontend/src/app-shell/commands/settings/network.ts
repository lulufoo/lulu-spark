import * as api from '../../../host/api.ts';
import type { NetworkConfig, PlanEntry, SettingsConfig } from '../../state/types.ts';

const STATUS_ID = 'settings-network-status';
const STATE_ID = 'settings-network-state';

type StatusState = { kind: 'ok' | 'error'; on: boolean; text: string };

export function loadNetworkPanel(cfg?: SettingsConfig) {
  const toggle = document.getElementById('settings-system-proxy');
  if (!(toggle instanceof HTMLInputElement)) return;
  toggle.checked = cfg?.network?.use_system_proxy === true;
  renderStatus(statusFor(cfg?.network));
}

export async function saveSystemProxySelection(value: boolean) {
  const toggle = document.getElementById('settings-system-proxy');
  try {
    const cfg = (await api.setConfig({ network: { use_system_proxy: value } })) as SettingsConfig;
    loadNetworkPanel(cfg);
  } catch (error) {
    // The persisted value is still the old one: reflect it instead of the click.
    if (toggle instanceof HTMLInputElement) toggle.checked = !value;
    renderStatus({
      kind: 'error',
      on: false,
      text: `Not saved: ${errMessage(error)}`,
      fix: 'switch restored to the saved value',
    });
  }
}

function statusFor(network?: NetworkConfig): StatusState {
  const on = network?.use_system_proxy === true;
  const plan = network?.effective?.plan;
  const scheme = (name: string, entry?: PlanEntry | null) =>
    entry?.host_port ? `${name} via ${entry.source} proxy ${entry.host_port}` : `${name} direct`;
  return {
    kind: 'ok',
    on,
    text: `Applied without restart: system proxy ${on ? 'on' : 'off'} — ${scheme('http', plan?.http)}, ${scheme('https', plan?.https)}.`,
  };
}

function renderStatus(state: StatusState & { fix?: string }) {
  const status = document.getElementById(STATUS_ID);
  const stateEl = document.getElementById(STATE_ID);
  if (!status || !stateEl) return;
  status.classList.toggle('is-ok', state.kind === 'ok');
  status.classList.toggle('is-on', state.kind === 'ok' && state.on);
  status.classList.toggle('is-error', state.kind === 'error');
  stateEl.textContent = state.fix
    ? `${state.text} `
    : state.text;
  if (state.fix) {
    const fix = document.createElement('span');
    fix.className = 'settings-network-fix';
    fix.textContent = state.fix;
    stateEl.appendChild(fix);
  }
}

function errMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
