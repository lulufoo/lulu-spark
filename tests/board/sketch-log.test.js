import { describe, expect, it, vi } from 'vitest';
import * as appLog from '../../frontend/src/host/app-log.ts';
import { elapsedMs, logSketch, SKETCH_BUSINESS } from '../../frontend/src/board/commands/sketch-log.ts';

describe('sketch log', () => {
  it('writes board_sketch events through logAppEvent', () => {
    const spy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    logSketch('sandbox.ready', { diagram_id: 'b_0123abcd', ms: 12 });
    expect(SKETCH_BUSINESS).toBe('board_sketch');
    expect(spy).toHaveBeenCalledWith({
      business: 'board_sketch',
      event: 'sandbox.ready',
      params: { diagram_id: 'b_0123abcd', ms: 12 },
      level: 'info',
    });
    spy.mockRestore();
  });

  it('treats a clock that moved backwards as zero', () => {
    expect(elapsedMs(performance.now() + 1000)).toBe(0);
  });
});
