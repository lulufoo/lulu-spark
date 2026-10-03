import * as api from '../../../host/api.ts';
import { setResult } from '../../state/settings/store.ts';
import { errMessage } from '../../state/types.ts';

type McpTool = { name: string; description?: string };
type McpToolGroup = { id: string; label?: string; tools?: McpTool[] };
type McpToolsSnapshot = {
  groups?: McpToolGroup[];
  enabled?: Record<string, string[]>;
  workbench_only_tools?: string[];
};

let toolsSnapshot: McpToolsSnapshot | null = null;
let persistBusy = false;
let persistQueued = false;

function catalogNames(snapshot: McpToolsSnapshot | null): string[] {
  return (snapshot?.groups ?? []).flatMap((group) =>
    (group.tools ?? []).map((tool) => tool.name).filter((name): name is string => Boolean(name)),
  );
}

function workbenchOnlyTools(snapshot: McpToolsSnapshot | null): Set<string> {
  return new Set(
    (snapshot?.workbench_only_tools ?? []).filter((name): name is string => Boolean(name)),
  );
}

/** Catalog names visible / selectable for the given MCP channel. */
function catalogNamesForChannel(snapshot: McpToolsSnapshot | null, channel: string): string[] {
  const only = workbenchOnlyTools(snapshot);
  if (channel === 'spark' || !only.size) return catalogNames(snapshot);
  return catalogNames(snapshot).filter((name) => !only.has(name));
}

function toolInputs() {
  return [
    ...document.querySelectorAll<HTMLInputElement>('#settings-mcp-tool-groups input[data-mcp-tool]'),
  ];
}

function selectedChannel() {
  return (
    (document.getElementById('settings-mcp-channel') as HTMLSelectElement | null)?.value ||
    'spark'
  );
}

/** Missing channel key = all catalog tools on (same as Host). Explicit `[]` = all off. */
function enabledNamesForChannel(snapshot: McpToolsSnapshot | null, channel: string) {
  const catalog = catalogNamesForChannel(snapshot, channel);
  const listed = snapshot?.enabled?.[channel];
  if (!Array.isArray(listed)) return new Set(catalog);
  return new Set(listed.filter((name) => catalog.includes(name)));
}

export function paintMcpToolGroups() {
  const groupsEl = document.getElementById('settings-mcp-tool-groups');
  const channel = selectedChannel();
  if (!groupsEl) return;
  groupsEl.replaceChildren();
  const groups = toolsSnapshot?.groups;
  if (!Array.isArray(groups) || !groups.length) return;
  const enabled = enabledNamesForChannel(toolsSnapshot, channel);
  const only = workbenchOnlyTools(toolsSnapshot);
  for (const group of groups) {
    const fieldset = document.createElement('fieldset');
    fieldset.className = 'settings-mcp-tool-group';
    const legend = document.createElement('legend');
    legend.textContent = group.label || group.id;
    fieldset.appendChild(legend);
    for (const tool of group.tools || []) {
      if (channel !== 'spark' && only.has(tool.name)) continue;
      const label = document.createElement('label');
      label.className = 'settings-mcp-tool';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.dataset.mcpTool = tool.name;
      input.checked = enabled.has(tool.name);
      const span = document.createElement('span');
      span.textContent = tool.name;
      label.appendChild(input);
      label.appendChild(span);
      fieldset.appendChild(label);
    }
    groupsEl.appendChild(fieldset);
  }
}

export async function loadMcpChannelTools() {
  try {
    toolsSnapshot = (await api.invoke('get_mcp_channel_tools')) as McpToolsSnapshot;
    paintMcpToolGroups();
    if (!toolInputs().length) {
      setResult('settings-result-mcp-tools', 'Load tools failed: empty catalog.', true);
    }
  } catch (e) {
    toolsSnapshot = null;
    paintMcpToolGroups();
    setResult('settings-result-mcp-tools', `Load tools failed: ${errMessage(e, String(e))}`, true);
  }
}

function rememberEnabled(channel: string, enabled: string[]) {
  if (!toolsSnapshot) return;
  const next = { ...(toolsSnapshot.enabled || {}) };
  const catalog = catalogNamesForChannel(toolsSnapshot, channel);
  const allOn =
    enabled.length === catalog.length && catalog.every((name) => enabled.includes(name));
  if (allOn) delete next[channel];
  else next[channel] = enabled;
  toolsSnapshot = { ...toolsSnapshot, enabled: next };
}

async function writeCurrentSelection() {
  const channel = selectedChannel();
  const boxes = toolInputs();
  if (!boxes.length) return false;
  const enabled = boxes
    .filter((el) => el.checked)
    .map((el) => el.dataset.mcpTool)
    .filter((name): name is string => Boolean(name));
  await api.invoke('set_mcp_channel_tools', { channel, enabled });
  rememberEnabled(channel, enabled);
  setResult('settings-result-mcp-tools', `Updated /mcp/${channel} tools.`);
  return true;
}

export async function persistMcpChannelTools() {
  if (persistBusy) {
    persistQueued = true;
    return;
  }
  persistBusy = true;
  setResult('settings-result-mcp-tools', '');
  try {
    do {
      persistQueued = false;
      if (!(await writeCurrentSelection())) {
        setResult('settings-result-mcp-tools', 'Update tools failed: tool list is empty.', true);
      }
    } while (persistQueued);
  } catch (e) {
    setResult('settings-result-mcp-tools', `Update tools failed: ${errMessage(e, String(e))}`, true);
  } finally {
    persistBusy = false;
  }
}

async function applyAllChecked(checked: boolean) {
  if (!toolInputs().length) await loadMcpChannelTools();
  const boxes = toolInputs();
  if (!boxes.length) {
    setResult('settings-result-mcp-tools', 'Update tools failed: tool list is empty.', true);
    return;
  }
  for (const box of boxes) box.checked = checked;
  await persistMcpChannelTools();
}

export async function selectAllMcpChannelTools() {
  await applyAllChecked(true);
}

export async function deselectAllMcpChannelTools() {
  await applyAllChecked(false);
}
