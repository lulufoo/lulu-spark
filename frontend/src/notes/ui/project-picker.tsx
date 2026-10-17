import { useMemo } from 'react';
import { Select } from '../../shared/components/select/index.ts';
import { enterProject } from '../commands/restore.ts';
import { sortedProjectFolders } from '../state/dir-tree.ts';
import { useHostState } from '../state/host.ts';

/**
 * Directory view: the project dropdown. It is always shown and always has a project selected:
 * every enabled project is an option, in the order Settings arranged, with its document count,
 * and there is no "All" (that would flatten the projects).
 */
export function ProjectPicker() {
  const host = useHostState();
  const titles = host.index.topicTitles;
  const data = host.index.data;
  const order = host.index.topicOrder;
  const folders = useMemo(() => sortedProjectFolders(titles, order), [titles, order]);
  const options = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const entry of Object.values(data || {})) {
      const folder = entry?.folder || 'unknown';
      counts[folder] = (counts[folder] || 0) + 1;
    }
    return folders.length
      ? folders.map((folder) => ({ value: folder, label: `${titles[folder] || folder} (${counts[folder] || 0})` }))
      : [{ value: '', label: 'No projects' }];
  }, [folders, titles, data]);
  return (
    <Select
      ariaLabel="Select project"
      menuMaxHeight={480}
      value={host.ui.activeTopic || folders[0] || ''}
      options={options}
      onChange={(value) => {
        if (value) enterProject(value);
      }}
    />
  );
}
