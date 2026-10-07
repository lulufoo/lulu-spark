import { logAppEvent } from '../host/app-log.ts';
import { parseTraceId } from '../router/notify-trace.ts';

export const LOGIN_BUSINESS = 'login';
export const LOGIN_EVENT_START = 'auth.start';
export const LOGIN_EVENT_ROUTE = 'route.to_business';
export const LOGIN_EVENT_EXCHANGE = 'auth.exchange';

export function logLoginHop(
  event: string,
  traceId: string | null | undefined,
  extra?: Record<string, unknown>,
): void {
  logAppEvent({
    business: LOGIN_BUSINESS,
    event,
    traceId: parseTraceId(traceId),
    params: extra ?? {},
  });
}
