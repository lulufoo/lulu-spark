import { invoke } from './api/transport.ts';

export function logAppEvent(input: {
  business: string;
  event: string;
  traceId?: string | null;
  params?: Record<string, unknown>;
  level?: 'info' | 'warn' | 'error';
}): void {
  try {
    void invoke('log_app_event', {
      business: input.business,
      event: input.event,
      traceId: input.traceId ?? undefined,
      params: input.params ?? {},
      level: input.level ?? 'info',
    }).catch(() => {
      // Host invoke is absent in unit tests / plain browsers.
    });
  } catch {
    // Host invoke is absent in unit tests / plain browsers.
  }
}
