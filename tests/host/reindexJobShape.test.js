import { describe, expect, it } from 'vitest';

/** Contract aligned with Python/Rust JobState JSON. */
const REQUIRED_KEYS = ['status', 'log'];

function assertJobShape(job) {
  for (const key of REQUIRED_KEYS) {
    expect(job).toHaveProperty(key);
  }
  expect(['idle', 'running', 'done', 'error']).toContain(job.status);
}

describe('reindexJobShape', () => {
  it('idle job has status and log keys', () => {
    assertJobShape({
      status: 'idle',
      started_at: null,
      finished_at: null,
      log: '',
    });
  });

  it('running job has status and log keys', () => {
    assertJobShape({
      status: 'running',
      started_at: '2026-05-20T00:00:00+00:00',
      finished_at: null,
      log: '同步仓库（并行）…',
    });
  });
});
