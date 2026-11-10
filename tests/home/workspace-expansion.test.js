// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { toggleWorkspaceExpansion, retainWorkspaceExpansion } from '../../frontend/src/home/state/workspace-expansion.ts';

describe('workspace expansion', () => {
  it('opens independently and collapses only the chosen group without mutating input', () => {
    const initial = [];
    const first = toggleWorkspaceExpansion(initial, 'a');
    const both = toggleWorkspaceExpansion(first, 'b');
    expect(initial).toEqual([]);
    expect(first).toEqual(['a']);
    expect(both).toEqual(['a', 'b']);
    expect(toggleWorkspaceExpansion(both, 'a')).toEqual(['b']);
  });

  it('drops removed IDs so re-enabled groups start collapsed', () => {
    const expanded = ['a', 'b'];
    const retained = retainWorkspaceExpansion(expanded, ['b']);
    expect(expanded).toEqual(['a', 'b']);
    expect(retained).toEqual(['b']);
    expect(retainWorkspaceExpansion(retained, ['a', 'b'])).toEqual(['b']);
    expect(retainWorkspaceExpansion(retained, [])).toEqual([]);
  });
});
