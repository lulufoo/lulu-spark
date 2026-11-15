import * as api from '../../host/api.ts';
import { setHomeState } from '../state/store.ts';

let unreadGen = 0;
let refreshSeq = 0;
let readLaterVersion = 0;
let readLaterPending = false;
const workspaceReadVersions = new Map<string, number>();
const pendingWorkspaceReads = new Set<string>();

function bumpWorkspaceReadVersion(id: string) {
  workspaceReadVersions.set(id, (workspaceReadVersions.get(id) ?? 0) + 1);
}

export function resetChannelUnread() {
  unreadGen += 1;
  refreshSeq += 1;
  readLaterVersion = 0;
  readLaterPending = false;
  workspaceReadVersions.clear();
  pendingWorkspaceReads.clear();
}

export async function refreshChannelUnread() {
  const gen = unreadGen;
  const seq = ++refreshSeq;
  const versions = new Map(workspaceReadVersions);
  const readLater = readLaterVersion;
  const [workspacesResult, channelResult] = await Promise.allSettled([
    api.getMessageWorkspaceUnread(),
    api.getMessageChannelUnread('read_later'),
  ]);
  if (gen !== unreadGen || seq !== refreshSeq) return;
  setHomeState((prev) => {
    let workspaceUnread = prev.workspaceUnread;
    if (workspacesResult.status === 'fulfilled') {
      workspaceUnread = Object.fromEntries(
        Object.entries(workspacesResult.value ?? {}).filter(([, unread]) => unread === true),
      );
      // A refresh from before or during a clear must not restore that dot.
      for (const [id, version] of workspaceReadVersions) {
        if (pendingWorkspaceReads.has(id) || version !== versions.get(id)) {
          if (prev.workspaceUnread[id]) workspaceUnread[id] = true;
          else delete workspaceUnread[id];
        }
      }
    }
    const canApplyReadLater = !readLaterPending && readLater === readLaterVersion;
    return {
      ...prev,
      workspaceUnread,
      channelUnread: {
        ...prev.channelUnread,
        read_later: channelResult.status === 'fulfilled' && canApplyReadLater
          ? channelResult.value === true
          : prev.channelUnread.read_later,
      },
    };
  });
}

export async function markHomeEntryRead(entry: string) {
  if (entry !== 'read-later' || readLaterPending) return;
  const gen = unreadGen;
  readLaterPending = true;
  readLaterVersion += 1;
  let cleared = false;
  try {
    await api.markMessageChannelRead('read_later');
    if (gen !== unreadGen) return;
    cleared = true;
    setHomeState((prev) => ({
      ...prev,
      channelUnread: { ...prev.channelUnread, read_later: false },
    }));
  } catch {
    // Keep the unread dot when the host could not persist the clear.
  } finally {
    if (gen === unreadGen) {
      readLaterPending = false;
      readLaterVersion += 1;
      if (cleared) await refreshChannelUnread();
    }
  }
}

export async function markHomeWorkspaceRead(id: string) {
  if (!id || pendingWorkspaceReads.has(id)) return;
  const gen = unreadGen;
  pendingWorkspaceReads.add(id);
  bumpWorkspaceReadVersion(id);
  let cleared = false;
  try {
    await api.markMessageWorkspaceRead(id);
    if (gen !== unreadGen) return;
    cleared = true;
    setHomeState((prev) => {
      const workspaceUnread = { ...prev.workspaceUnread };
      delete workspaceUnread[id];
      return { ...prev, workspaceUnread };
    });
  } catch {
    // A failed clear must not hide this or any other workspace's unread dot.
  } finally {
    if (gen === unreadGen) {
      pendingWorkspaceReads.delete(id);
      bumpWorkspaceReadVersion(id);
      if (cleared) await refreshChannelUnread();
    }
  }
}
