// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { copyCurrentSessionId } from '../../frontend/src/home/commands/copy-session-id.ts';
import { resetHomeState, setHomeState } from '../../frontend/src/home/state/store.ts';

describe('copyCurrentSessionId', () => {
  beforeEach(() => {
    resetHomeState();
  });

  afterEach(() => {
    resetHomeState();
  });

  it('writes the current session id', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    setHomeState((prev) => ({ ...prev, currentSessionId: 'sess_abc' }));
    await expect(copyCurrentSessionId()).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('sess_abc');
  });

  it('returns false without a session id', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    await expect(copyCurrentSessionId()).resolves.toBe(false);
    expect(writeText).not.toHaveBeenCalled();
  });

  it('returns false when clipboard is missing', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: undefined,
    });
    setHomeState((prev) => ({ ...prev, currentSessionId: 'sess_abc' }));
    await expect(copyCurrentSessionId()).resolves.toBe(false);
  });
});
