import { state } from '../state/host.ts';
import { createFloatingListSelect } from '../../shared/floating-list-select.ts';
import { clearProjectSelection, selectProject } from '../commands/sidebar.ts';

/**
 * Directory view: the project dropdown, shaped like main's Knowledge library picker.
 * Every enabled project is an option with its document count; there is no "All" (that would
 * flatten the projects), so an unselected workspace shows the placeholder.
 */
export function renderProjectPicker(parent: HTMLElement) {
  const counts: Record<string, number> = {};
  for (const entry of Object.values(state.index.data || {})) {
    const folder = entry?.folder || 'unknown';
    counts[folder] = (counts[folder] || 0) + 1;
  }
  const titles = state.index.topicTitles;
  const folders = Object.keys(titles).sort((a, b) => (titles[a] || a).localeCompare(titles[b] || b));
  if (folders.length === 0) return;

  const options = [
    { value: '', label: 'Select a project' },
    ...folders.map((folder) => ({
      value: folder,
      label: `${titles[folder] || folder} (${counts[folder] || 0})`,
    })),
  ];
  const { picker } = createFloatingListSelect({
    ariaLabel: 'Select project',
    pickerClass: 'knowledge-repo-picker',
    menuMaxHeight: 480,
    value: state.ui.activeTopic || '',
    options,
    onSelect: (value: string) => (value ? selectProject(value) : clearProjectSelection()),
  });
  parent.appendChild(picker);
}
