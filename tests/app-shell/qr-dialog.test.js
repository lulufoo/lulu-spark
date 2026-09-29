import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('qr-dialog', () => {
  it('does not keep the QR page modules', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/ui/qr-dialog.tsx'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/commands/qr-dialog.ts'))).toBe(false);
  });
});
