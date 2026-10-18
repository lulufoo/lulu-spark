import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchGet = vi.fn();

vi.mock('../../frontend/src/host/api/transport.ts', () => ({
  getReadDriver: () => ({ fetchGet }),
}));

const { searchChats } = await import('../../frontend/src/host/api/search.ts');

const lastUrl = () => new URL(fetchGet.mock.calls.at(-1)[0], 'http://x');

describe('searchChats query', () => {
  beforeEach(() => {
    fetchGet.mockReset();
    fetchGet.mockResolvedValue({ json: async () => ({ hits: [] }) });
  });

  it('asks /api/search-chats with q and limit, and nothing else', async () => {
    await searchChats('login', 8);
    expect(lastUrl().pathname).toBe('/api/search-chats');
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'login', limit: '8' });

    await searchChats('login');
    expect(Object.fromEntries(lastUrl().searchParams)).toEqual({ q: 'login', limit: '20' });
  });

  it('keeps %, _ and a backslash in the query intact', async () => {
    await searchChats('50%_a\\b', 8);
    expect(lastUrl().searchParams.get('q')).toBe('50%_a\\b');
  });

  it('returns the parsed JSON of the response', async () => {
    const hits = [{ session_id: 's1', title: 'Login', updated_at: 1 }];
    fetchGet.mockResolvedValue({ json: async () => ({ hits }) });
    expect(await searchChats('login', 8)).toEqual({ hits });
  });
});
