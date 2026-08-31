import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('README TestSandbox docs', () => {
  it('documents TestSandbox config roots and default ports', () => {
    const text = readFileSync(join(root, 'README.md'), 'utf8');
    expect(text).toMatch(/TestSandbox/);
    expect(text).toMatch(/lulu-workbench-sandbox/);
    expect(text).toMatch(/18765/);
    expect(text).toMatch(/19876/);
    expect(text).toMatch(/8765/);
    expect(text).toMatch(/9876/);
    expect(text).not.toMatch(/TEST_MODE=1/);
    expect(text).not.toMatch(/cache-first/);
  });

  it('does not document TestSandbox inside config.toml examples as an in-file flag', () => {
    const text = readFileSync(join(root, 'README.md'), 'utf8');
    const configExample = text.match(/```toml[\s\S]*?```/g);
    expect(configExample?.[0] ?? '').not.toMatch(/TestSandbox/);
  });
});
