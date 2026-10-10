// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/read-later/commands/list.ts', () => ({
  loadReadLaterEntries: vi.fn(),
  openExternalUrl: vi.fn(),
}));
vi.mock('../../frontend/src/home/commands/hub.ts', () => ({
  showActionError: vi.fn(),
}));

import { loadReadLaterEntries, openExternalUrl } from '../../frontend/src/read-later/commands/list.ts';
import { showActionError } from '../../frontend/src/home/commands/hub.ts';
import {
  onReadLaterLinkClick,
  openReadLaterById,
  readLaterIdFromHref,
} from '../../frontend/src/home/commands/open-read-later-link.ts';
import { onChatLinkClick } from '../../frontend/src/home/commands/open-chat-link.ts';

const ID = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const URL = 'https://example.com/saved';

function clickOn(handler, html, selector) {
  const root = document.createElement('div');
  root.innerHTML = html;
  const event = { target: root.querySelector(selector), preventDefault: vi.fn() };
  handler(event);
  return event;
}

function flush() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  vi.mocked(loadReadLaterEntries).mockReset();
  vi.mocked(openExternalUrl).mockReset();
  vi.mocked(showActionError).mockReset();
});

describe('readLaterIdFromHref', () => {
  it('accepts read-later:<32-hex>', () => {
    expect(readLaterIdFromHref(`read-later:${ID}`)).toBe(ID);
    expect(readLaterIdFromHref(`  read-later:${ID} `)).toBe(ID);
  });

  it('rejects anything that is not a 32-hex read-later link', () => {
    for (const bad of [
      'read-later:',
      'read-later:abc123',
      `read-later:${ID}/x`,
      `note:${ID}`,
      `Read-later:${ID}`,
      '',
      null,
      undefined,
    ]) {
      expect(readLaterIdFromHref(bad)).toBeNull();
    }
  });
});

describe('openReadLaterById', () => {
  it('opens the saved url for a known id', async () => {
    vi.mocked(loadReadLaterEntries).mockResolvedValue([{ id: ID, url: URL, title: 'Saved' }]);
    vi.mocked(openExternalUrl).mockResolvedValue();
    await openReadLaterById(ID);
    expect(loadReadLaterEntries).toHaveBeenCalledOnce();
    expect(openExternalUrl).toHaveBeenCalledWith(URL);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('reports when the id is not in the list', async () => {
    vi.mocked(loadReadLaterEntries).mockResolvedValue([]);
    await openReadLaterById(ID);
    expect(openExternalUrl).not.toHaveBeenCalled();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open read-later/);
  });
});

describe('onReadLaterLinkClick', () => {
  it('intercepts a read-later: link and opens the url', async () => {
    vi.mocked(loadReadLaterEntries).mockResolvedValue([{ id: ID, url: URL }]);
    vi.mocked(openExternalUrl).mockResolvedValue();
    const event = clickOn(
      onReadLaterLinkClick,
      `<p><a href="read-later:${ID}"><b>Title</b></a></p>`,
      'b',
    );
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(openExternalUrl).toHaveBeenCalledWith(URL);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('leaves every other link alone', async () => {
    const web = clickOn(onReadLaterLinkClick, '<a href="https://example.com">x</a>', 'a');
    const note = clickOn(onReadLaterLinkClick, `<a href="note:${ID}">x</a>`, 'a');
    const plain = clickOn(onReadLaterLinkClick, '<span>no link</span>', 'span');
    for (const event of [web, note, plain]) expect(event.preventDefault).not.toHaveBeenCalled();
    await flush();
    expect(loadReadLaterEntries).not.toHaveBeenCalled();
  });

  it('reports a malformed read-later: link without listing entries', async () => {
    const event = clickOn(onReadLaterLinkClick, '<a href="read-later:zzz">x</a>', 'a');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(loadReadLaterEntries).not.toHaveBeenCalled();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open read-later/);
  });
});

describe('onChatLinkClick', () => {
  it('routes read-later: links to the read-later flow', async () => {
    vi.mocked(loadReadLaterEntries).mockResolvedValue([{ id: ID, url: URL }]);
    vi.mocked(openExternalUrl).mockResolvedValue();
    clickOn(onChatLinkClick, `<a href="read-later:${ID}">x</a>`, 'a');
    await flush();
    expect(openExternalUrl).toHaveBeenCalledWith(URL);
  });
});
