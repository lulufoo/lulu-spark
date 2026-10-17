/** Fired when workspaces or projects were added, renamed, disabled or enabled. */
export const REGISTRY_CHANGED_EVENT = 'cta:registry-changed';

export function announceRegistryChange() {
  document.dispatchEvent(new CustomEvent(REGISTRY_CHANGED_EVENT));
}
