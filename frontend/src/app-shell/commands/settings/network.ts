import * as api from '../../../host/api.ts';
import type { NetworkConfig, PlanEntry, SettingsConfig } from '../../state/types.ts';
import { setResult } from '../../state/settings/store.ts';

const RESULT_ID = 'settings-network-result';

export function loadNetworkPanel(cfg?: SettingsConfig) {
  const toggle = document.getElementById('settings-system-proxy');
  if (!(toggle instanceof HTMLInputElement)) return;
  toggle.checked = cfg?.network?.use_system_proxy === true;
  setResult(RESULT_ID, describeApplied(cfg?.network), false);
}

export async function saveSystemProxySelection(value: boolean) {
  const toggle = document.getElementById('settings-system-proxy');
  try {
    const cfg = (await api.setConfig({ network: { use_system_proxy: value } })) as SettingsConfig;
    loadNetworkPanel(cfg);
    setResult(RESULT_ID, describeApplied(cfg?.network), false);
  } catch (error) {
    // The persisted value is still the old one: reflect it instead of the click.
    if (toggle instanceof HTMLInputElement) toggle.checked = !value;
    setResult(RESULT_ID, `Not saved: ${errMessage(error)}`, true);
  }
}

function describeApplied(network?: NetworkConfig): string {
  const on = network?.use_system_proxy === true;
  const plan = network?.effective?.plan;
  const scheme = (name: string, entry?: PlanEntry | null) =>
    entry?.host_port ? `${name} via ${entry.source} proxy ${entry.host_port}` : `${name} direct`;
  return `Applied without restart: system proxy ${on ? 'on' : 'off'}; ${scheme('http', plan?.http)}, ${scheme('https', plan?.https)}.`;
}

function errMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
