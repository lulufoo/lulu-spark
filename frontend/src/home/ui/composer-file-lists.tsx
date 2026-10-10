import type { HubStagedEntry, HubWorkspaceFile } from '../state/store.ts';
import { StagedList } from './staged-list.tsx';
import { useExclusiveDetailsPair } from './use-dismissible-details.ts';
import { WorkspaceList } from './workspace-list.tsx';

export function ComposerFileLists({
  workspace,
  staged,
  canRemove,
  onOpenWorkspace,
  onOpenStaged,
  onRemoveWorkspace,
  onRemoveStaged,
}: {
  workspace: HubWorkspaceFile[];
  staged: HubStagedEntry[];
  canRemove: boolean;
  onOpenWorkspace: (item: HubWorkspaceFile) => void;
  onOpenStaged: (item: HubStagedEntry) => void;
  onRemoveWorkspace: (path: string) => void;
  onRemoveStaged: (id: string) => void;
}) {
  const { bind } = useExclusiveDetailsPair();
  return (
    <>
      <WorkspaceList
        items={workspace}
        canRemove={canRemove}
        onOpen={onOpenWorkspace}
        onRemove={onRemoveWorkspace}
        {...bind('workspace')}
      />
      <StagedList
        items={staged}
        canRemove={canRemove}
        onOpen={onOpenStaged}
        onRemove={onRemoveStaged}
        {...bind('staged')}
      />
    </>
  );
}
