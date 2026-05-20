import { describe, expect, it } from 'vitest';
import {
  REINDEX_INVOKE_MAP,
  resolveReindexInvoke,
} from '../frontend/js/searchApiInvokeMap.js';

const REINDEX_KEYS = [
  'reindexKnowledge',
  'reindexWorkbench',
  'reindexKbRepo',
  'getReindexStatus',
  'getReindexWorkbenchStatus',
];

describe('searchApiInvokeMap', () => {
  it('covers all 5 P3 reindex invoke keys', () => {
    for (const key of REINDEX_KEYS) {
      expect(REINDEX_INVOKE_MAP[key]?.cmd, key).toBeTruthy();
    }
    expect(Object.keys(REINDEX_INVOKE_MAP)).toHaveLength(5);
  });

  it('resolveReindexInvoke maps kb repo body', () => {
    expect(resolveReindexInvoke('reindexKbRepo', { repo: 'lulufoo/foo' })).toEqual({
      cmd: 'reindex_kb_repo',
      args: { repo: 'lulufoo/foo' },
    });
  });

  it('resolveReindexInvoke maps status commands without args', () => {
    expect(resolveReindexInvoke('getReindexStatus')).toEqual({
      cmd: 'get_reindex_status',
      args: {},
    });
    expect(resolveReindexInvoke('getReindexWorkbenchStatus')).toEqual({
      cmd: 'get_reindex_workbench_status',
      args: {},
    });
  });
});
