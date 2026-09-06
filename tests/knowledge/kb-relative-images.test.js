// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { fetchKbAssetAsBlobUrl } = vi.hoisted(() => ({
  fetchKbAssetAsBlobUrl: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchKbAssetAsBlobUrl,
}));

import { state } from '../../frontend/src/host/state.ts';
import { hydrateKbRelativeImages, revokeKbBlobUrls } from '../../frontend/src/knowledge/commands/viewer/images.ts';

describe('hydrateKbRelativeImages', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    state.viewer.kbRepo = 'owner/repo';
    state.viewer.kbPath = 'docs/a.md';
    fetchKbAssetAsBlobUrl.mockReset();
    fetchKbAssetAsBlobUrl.mockResolvedValue('blob:http://local/kb-img');
  });

  afterEach(() => {
    revokeKbBlobUrls();
    container.remove();
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
  });

  it('rewrites relative img src to blob URL', async () => {
    container.innerHTML = '<img src="note.png" alt="n">';
    await hydrateKbRelativeImages(container);
    expect(fetchKbAssetAsBlobUrl).toHaveBeenCalledWith('owner/repo', 'docs/a.md', 'note.png');
    expect(container.querySelector('img').getAttribute('src')).toBe('blob:http://local/kb-img');
  });

  it('rewrites ./ relative img src', async () => {
    container.innerHTML = '<img src="./note.png" alt="n">';
    await hydrateKbRelativeImages(container);
    expect(fetchKbAssetAsBlobUrl).toHaveBeenCalledWith('owner/repo', 'docs/a.md', './note.png');
    expect(container.querySelector('img').getAttribute('src')).toBe('blob:http://local/kb-img');
  });

  it('leaves https img src alone', async () => {
    container.innerHTML = '<img src="https://example.com/a.png" alt="n">';
    await hydrateKbRelativeImages(container);
    expect(fetchKbAssetAsBlobUrl).not.toHaveBeenCalled();
    expect(container.querySelector('img').getAttribute('src')).toBe('https://example.com/a.png');
  });

  it('sets alt from href when asset load fails', async () => {
    fetchKbAssetAsBlobUrl.mockRejectedValue(new Error('missing'));
    container.innerHTML = '<img src="gone.png">';
    await hydrateKbRelativeImages(container);
    expect(container.querySelector('img').getAttribute('src')).toBe('gone.png');
    expect(container.querySelector('img').getAttribute('alt')).toBe('gone.png');
  });
});
