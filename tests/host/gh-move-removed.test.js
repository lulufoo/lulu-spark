import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const workbenchApiPath = join(repoRoot, 'frontend/src/host/api/workbench.ts');
const apiPath = join(repoRoot, 'frontend/src/host/api.ts');

const KEPT_WORKBENCH_EXPORTS = [
  'checkFileExists',
  'checkWorkbenchRoot',
  'clearNoteDraft',
  'commitFiles',
  'createNote',
  'deleteEntry',
  'fetchAnnotation',
  'fetchAnnotationsSummary',
  'fetchConfig',
  'fetchDiffStatus',
  'fetchDocHighlights',
  'fetchFileContent',
  'fetchIndex',
  'fetchLinkTitle',
  'fetchNotesAssetAsBlobUrl',
  'fetchRepoDirs',
  'fetchTagsRegistry',
  'fetchTopics',
  'getDraft',
  'getNoteDraft',
  'ghDelete',
  'inferGithubUserUrl',
  'moveToProject',
  'pullProject',
  'reorderComments',
  'revertFile',
  'saveDraft',
  'saveFile',
  'saveNoteDraft',
  'setConfig',
  'setDone',
  'setImportance',
  'settleComment',
  'tagAttach',
  'tagDetach',
  'tagUpdateValue',
  'updateComments',
  'updateDocHighlights',
  'updateHighlight',
  'updateLinks',
];

describe('ghMove removed from host API', () => {
  it('does not export ghMove from workbench.ts or re-export it from api.ts', () => {
    expect(existsSync(workbenchApiPath)).toBe(true);
    expect(existsSync(apiPath)).toBe(true);
    const workbench = readFileSync(workbenchApiPath, 'utf8');
    const api = readFileSync(apiPath, 'utf8');
    expect(workbench).not.toMatch(
      /export\s+(?:async\s+)?(?:function ghMove|const ghMove|class ghMove|type ghMove)\b/,
    );
    expect(workbench).not.toMatch(/export\s+\{[^}]*\bghMove\b/);
    expect(workbench).not.toMatch(/\bghMove\b/);
    expect(api).not.toMatch(
      /export\s+(?:async\s+)?(?:function ghMove|const ghMove|class ghMove|type ghMove)\b/,
    );
    expect(api).not.toMatch(/export\s+\{[^}]*\bghMove\b/);
    expect(api).not.toMatch(/\bghMove\b/);
  });

  it('keeps the other workbench host APIs, including ghDelete', () => {
    expect(existsSync(workbenchApiPath)).toBe(true);
    expect(existsSync(apiPath)).toBe(true);
    const workbench = readFileSync(workbenchApiPath, 'utf8');
    const api = readFileSync(apiPath, 'utf8');
    for (const name of KEPT_WORKBENCH_EXPORTS) {
      expect(workbench).toMatch(
        new RegExp(
          String.raw`export\s+(?:async\s+)?(?:function ${name}|const ${name}|class ${name}|type ${name})\b`,
        ),
      );
      expect(api).toMatch(new RegExp(String.raw`\b${name}\b`));
    }
  });
});
