import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('ConvertDialog QR tab', () => {
  it('does not import ConvertDialog or convertStore; those modules are gone', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/ui/convert-dialog.tsx'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/state/convert.ts'))).toBe(false);
  });
});
