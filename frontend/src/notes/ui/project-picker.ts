import { state } from '../state/host.ts';
import { createFloatingListSelect } from '../../shared/floating-list-select.ts';
import { sortedProjectFolders } from '../state/dir-tree.ts';
import { selectProject } from '../commands/sidebar.ts';

/**
 * Directory view: the project dropdown, shaped like main's Knowledge library picker. It is always
 * shown and always has a project selected: every enabled project is an option with its document
 * count, and there is no "All" (that would flatten the projects).
 */
export function renderProjectPicker(parent: HTMLElement) {
  const counts: Record<string, number> = {};
  for (const entry of Object.values(state.index.data || {})) {
    const folder = entry?.folder || 'unknown';
    counts[folder] = (counts[folder] || 0) + 1;
  }
  const titles = state.index.topicTitles;
  const folders = sortedProjectFolders(titles);

  const options = folders.length
    ? folders.map((folder) => ({
        value: folder,
        label: `${titles[folder] || folder} (${counts[folder] || 0})`,
      }))
    : [{ value: '', label: 'No projects' }];
  const { picker } = createFloatingListSelect({
    ariaLabel: 'Select project',
    pickerClass: 'knowledge-repo-picker',
    menuMaxHeight: 480,
    value: state.ui.activeTopic || folders[0] || '',
    options,
    onSelect: (value: string) => {
      if (value) selectProject(value);
    },
  });
  parent.appendChild(picker);
}
