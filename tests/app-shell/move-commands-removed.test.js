import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const moveCommandPath = join(repoRoot, 'frontend/src/app-shell/commands/move-dialog.ts');
const sparkApiPath = join(repoRoot, 'frontend/src/host/api/spark.ts');
const apiPath = join(repoRoot, 'frontend/src/host/api.ts');

describe('Move document command functions removed', () => {
  it('deletes move-dialog.ts and the two command functions', () => {
    expect(existsSync(moveCommandPath)).toBe(false);
    const product = listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
      .map((abs) => readFileSync(abs, 'utf8'))
      .join('\n');
    expect(product).not.toMatch(/\bdoMoveDoc\b/);
    expect(product).not.toMatch(/\bdoDeleteDoc\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function doMoveDoc|const doMoveDoc|class doMoveDoc|type doMoveDoc)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bdoMoveDoc\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function doDeleteDoc|const doDeleteDoc|class doDeleteDoc|type doDeleteDoc)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bdoDeleteDoc\b/);
  });

  it('does not delete ghDelete', () => {
    expect(existsSync(sparkApiPath)).toBe(true);
    expect(existsSync(apiPath)).toBe(true);
    const spark = readFileSync(sparkApiPath, 'utf8');
    const api = readFileSync(apiPath, 'utf8');
    expect(spark).toMatch(/export async function ghDelete\b/);
    expect(api).toMatch(/\bghDelete\b/);
  });
});
