/** Tiny external store so leftover callers can still `openX()` without finding `#id`. */
export function createModuleStore<T>(initial: T) {
  let value = initial;
  const listeners = new Set<() => void>();

  return {
    getSnapshot: () => value,
    set(next: T | ((prev: T) => T)) {
      const resolved = typeof next === 'function' ? (next as (prev: T) => T)(value) : next;
      if (Object.is(resolved, value)) return;
      value = resolved;
      listeners.forEach((fn) => fn());
    },
    subscribe(fn: () => void) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
  };
}
