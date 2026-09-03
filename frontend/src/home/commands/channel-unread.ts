import * as api from '../../host/api.ts';
import { setHomeState } from '../state/store.ts';

const UNREAD_CHANNELS = ['notes', 'read_later', 'todos'] as const;
const ENTRY_CHANNEL: Record<string, (typeof UNREAD_CHANNELS)[number]> = {
  workbench: 'notes',
  'read-later': 'read_later',
  'todo-tasks': 'todos',
};

let unreadGen = 0;

export function resetChannelUnread() {
  unreadGen += 1;
}

export async function refreshChannelUnread() {
  const gen = unreadGen;
  const next = { notes: false, read_later: false, todos: false };
  for (const channel of UNREAD_CHANNELS) {
    try {
      next[channel] = (await api.getMessageChannelUnread(channel)) === true;
    } catch {
      next[channel] = false;
    }
    if (gen !== unreadGen) return;
  }
  setHomeState((prev) => ({ ...prev, channelUnread: next }));
}

export async function markHomeEntryRead(entry: string) {
  const channel = ENTRY_CHANNEL[entry];
  if (!channel) return;
  try {
    await api.markMessageChannelRead(channel);
  } catch {
    return;
  }
  setHomeState((prev) => ({
    ...prev,
    channelUnread: { ...prev.channelUnread, [channel]: false },
  }));
}
