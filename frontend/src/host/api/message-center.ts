import { readGet, writePost } from './transport.ts';

export async function getMessageChannelUnread(channel: string) {
  return readGet(
    `/api/message-channel-unread?channel=${encodeURIComponent(channel)}`,
  );
}

export async function markMessageChannelRead(channel: string) {
  return writePost('/api/mark-message-channel-read', { channel });
}
