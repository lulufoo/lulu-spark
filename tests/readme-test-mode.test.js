import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const readmePath = join(repoRoot, 'README.md');

function readme() {
  return readFileSync(readmePath, 'utf8');
}

describe('README TEST_MODE and migration docs', () => {
  it('documents TEST_MODE=1 three runtime states', () => {
    const text = readme();
    expect(text).toMatch(/TEST_MODE=1/);
    expect(text).toMatch(/TestSandbox|automated test sandbox/i);
    expect(text).toMatch(/cache-first|manual cache-first/i);
    expect(text).toMatch(/prod|正式运行|未设.*TEST_MODE/i);
  });

  it('documents deploy → migrate-local-state → corpus git commit order', () => {
    const text = readme();
    expect(text).toMatch(/deploy/i);
    expect(text).toMatch(/scripts\/migrate-local-state/);
    expect(text).toMatch(/git commit/i);
    const deployIdx = text.search(/deploy/i);
    const migrateIdx = text.indexOf('scripts/migrate-local-state');
    const commitIdx = text.search(/git commit/i);
    expect(deployIdx).toBeGreaterThanOrEqual(0);
    expect(migrateIdx).toBeGreaterThan(deployIdx);
    expect(commitIdx).toBeGreaterThan(migrateIdx);
  });

  it('documents AC-5 manual cache-first verification steps', () => {
    const text = readme();
    expect(text).toMatch(/AC-5|cache-first.*验证|人工.*cache/i);
    expect(text).toMatch(/不一致|intentionally different|故意不一致/i);
    expect(text).toMatch(/读.*cache|read.*cache/i);
    expect(text).toMatch(/写.*语料仓|write.*workbench_knowledge_root|写入.*语料仓/i);
    expect(text).toMatch(/unset TEST_MODE/);
  });

  it('documents unset TEST_MODE restores prod corpus-only read semantics', () => {
    const text = readme();
    expect(text).toMatch(/unset TEST_MODE/);
    expect(text).toMatch(/只读.*语料仓|prod.*只读|corpus.*read-only/i);
  });

  it('does not document TEST_MODE in config.toml (NG constraint)', () => {
    const text = readme();
    const configExample = text.match(/```toml[\s\S]*?```/);
    expect(configExample?.[0] ?? '').not.toMatch(/TEST_MODE/);
  });
});
