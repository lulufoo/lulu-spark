import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const src = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), '../../frontend/src/knowledge/ui/tree-delete-dialog.tsx'),
  'utf8',
);

describe('knowledge tree delete dialog', () => {
  it('requires typing CONFIRM before delete is enabled', () => {
    expect(src).toContain("Type <code>CONFIRM</code> to delete");
    expect(src).toContain('Enter CONFIRM here');
    expect(src).toContain("disabled={typed !== 'CONFIRM'}");
    expect(src).toContain("if (typed !== 'CONFIRM') return");
    expect(src).toContain('Confirm delete');
  });
});
