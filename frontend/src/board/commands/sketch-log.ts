import { logAppEvent } from '../../host/app-log.ts';

export const SKETCH_BUSINESS = 'board_sketch';

export function elapsedMs(startedAt: number): number {
  const ms = Math.round(performance.now() - startedAt);
  return ms < 0 ? 0 : ms;
}

export function logSketch(
  event: string,
  params: Record<string, unknown> = {},
  level: 'info' | 'warn' | 'error' = 'info',
): void {
  logAppEvent({
    business: SKETCH_BUSINESS,
    event,
    params,
    level,
  });
}
