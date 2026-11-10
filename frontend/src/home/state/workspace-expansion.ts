/** Expand independently: opening one workspace never closes another. */
export function toggleWorkspaceExpansion(expanded: string[], id: string): string[] {
  return expanded.includes(id) ? expanded.filter((value) => value !== id) : [...expanded, id];
}

/** Removed or disabled workspaces must not reopen if their id is later reused. */
export function retainWorkspaceExpansion(expanded: string[], availableIds: string[]): string[] {
  return expanded.filter((id) => availableIds.includes(id));
}
