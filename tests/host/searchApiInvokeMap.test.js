import { describe, expect, it } from 'vitest';
import {
  REINDEX_INVOKE_MAP,
  resolveReindexInvoke,
} from '../../frontend/src/host/searchApiInvokeMap.ts';

const REINDEX_KEYS = [
  'reindexAll',
  'getReindexAllStatus',
];

describe('searchApiInvokeMap', () => {
  it('covers all P3 reindex invoke keys', () => {
    for (const key of REINDEX_KEYS) {
      expect(REINDEX_INVOKE_MAP[key]?.cmd, key).toBeTruthy();
    }
    expect(Object.keys(REINDEX_INVOKE_MAP)).toHaveLength(REINDEX_KEYS.length);
  });

  it('drops the per-field rebuild commands removed with the header control', () => {
    expect(REINDEX_INVOKE_MAP.reindexKnowledge).toBeUndefined();
    expect(REINDEX_INVOKE_MAP.reindexSpark).toBeUndefined();
    expect(REINDEX_INVOKE_MAP.getReindexSparkStatus).toBeUndefined();
  });

  it('resolveReindexInvoke maps header rebuild and status commands without args', () => {
    expect(resolveReindexInvoke('reindexAll')).toEqual({ cmd: 'reindex_all', args: {} });
    expect(resolveReindexInvoke('getReindexAllStatus')).toEqual({
      cmd: 'get_reindex_all_status',
      args: {},
    });
  });
});
