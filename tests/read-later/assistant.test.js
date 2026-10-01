import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readMainSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('Read Later hub preview retired', () => {
  it('removes assistant.tsx and its boot adapter registration', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/read-later/ui/assistant.tsx'))).toBe(
      false,
    );
    const mainJs = readMainSource();
    expect(mainJs).not.toMatch(/createReadLaterContentAdapter/);
    expect(mainJs).not.toMatch(/mountHomeEntryShell\s*\(/);
    expect(mainJs).not.toMatch(/mountReadLaterAssistantWidget\s*\(\s*document\.body\b/);
  });

  it('keeps the read-later directory for the existing list dialog', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/read-later'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/read-later/ui/dialog.tsx'))).toBe(true);
  });
});
