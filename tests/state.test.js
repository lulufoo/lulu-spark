import { describe, it, expect, beforeEach } from 'vitest';
import { state } from '../frontend/js/host/state.js';

describe('state initial values', () => {
  it('state.index.filteredGroups 初始值为 []', () => {
    expect(Array.isArray(state.index.filteredGroups)).toBe(true);
    expect(state.index.filteredGroups).toHaveLength(0);
  });

  it('state.ui.activeTopic 初始值为 null', () => {
    expect(state.ui.activeTopic).toBeNull();
  });
});
