// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';

import { copyMessageText } from '../../frontend/src/home/commands/copy-message.ts';

describe('copyMessageText', () => {
  it('writes the message text', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    await expect(copyMessageText('hello')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('hello');
  });

  it('returns false without text', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    await expect(copyMessageText('')).resolves.toBe(false);
    expect(writeText).not.toHaveBeenCalled();
  });

  it('returns false when clipboard is missing', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: undefined,
    });
    await expect(copyMessageText('hello')).resolves.toBe(false);
  });
});
