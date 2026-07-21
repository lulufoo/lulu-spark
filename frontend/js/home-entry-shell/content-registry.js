/**
 * ContentRegistry: contentKey → business content adapter.
 * Mutable map; unregistered keys resolve to undefined.
 */

export function createContentRegistry() {
  /** @type {Map<string, unknown>} */
  const adapters = new Map();

  return {
    /**
     * @param {string} contentKey
     * @param {unknown} adapter
     */
    register(contentKey, adapter) {
      adapters.set(contentKey, adapter);
    },

    /**
     * @param {string} contentKey
     * @returns {unknown}
     */
    get(contentKey) {
      return adapters.get(contentKey);
    },
  };
}
