/**
 * ContentRegistry: contentKey → business content adapter.
 * Mutable map; unregistered keys resolve to undefined.
 */

export function createContentRegistry() {
  const adapters = new Map<string, unknown>();

  return {
    register(contentKey: string, adapter: unknown) {
      adapters.set(contentKey, adapter);
    },

    get(contentKey: string) {
      return adapters.get(contentKey);
    },
  };
}
