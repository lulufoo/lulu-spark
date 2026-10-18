import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchGet = vi.fn();

vi.mock('../../frontend/src/host/api/transport.ts', () => ({
  getReadDriver: () => ({ fetchGet }),
}));

const { searchSpark } = await import('../../frontend/src/host/api/search.ts');

const lastUrl = () => new URL(fetchGet.mock.calls.at(-1)[0], 'http://x');

describe('searchSpark query', () => {
  beforeEach(() => {
    fetchGet.mockReset();
    fetchGet.mockResolvedValue({ json: async () => ({ hits: [] }) });
  });

  it('keeps the existing call shape: q, limit, and workspace_id only when given', async () => {
    await searchSpark('alpha', 8, 'memory');
    expect(lastUrl().pathname).toBe('/api/search-spark');
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'alpha', limit: '8', workspace_id: 'memory' });

    await searchSpark('alpha');
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'alpha', limit: '10' });
  });

  it('asks for every workspace with all_workspaces=1 and sends no workspace_id', async () => {
    await searchSpark('alpha', 8, '', { allWorkspaces: true });
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'alpha', limit: '8', all_workspaces: '1' });
  });

  it('lets allWorkspaces win over a workspace id', async () => {
    await searchSpark('alpha', 8, 'memory', { allWorkspaces: true });
    expect(lastUrl().searchParams.get('all_workspaces')).toBe('1');
    expect(lastUrl().searchParams.has('workspace_id')).toBe(false);
  });

  it('treats allWorkspaces: false like not passing it', async () => {
    await searchSpark('alpha', 8, 'memory', { allWorkspaces: false });
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'alpha', limit: '8', workspace_id: 'memory' });
  });

  it('returns the parsed JSON of the response', async () => {
    fetchGet.mockResolvedValue({ json: async () => ({ hits: [{ id: 'd', workspace_id: 'w' }] }) });
    expect(await searchSpark('alpha', 8, '', { allWorkspaces: true })).toEqual({ hits: [{ id: 'd', workspace_id: 'w' }] });
  });
});
