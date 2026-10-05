import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sparkApiPath = join(repoRoot, 'frontend/src/host/api/spark.ts');
const apiPath = join(repoRoot, 'frontend/src/host/api.ts');

const KEPT_SPARK_EXPORTS = [
  'checkFileExists',
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
  'moveToProject',
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
  it('does not export ghMove from spark.ts or re-export it from api.ts', () => {
    expect(existsSync(sparkApiPath)).toBe(true);
    expect(existsSync(apiPath)).toBe(true);
    const spark = readFileSync(sparkApiPath, 'utf8');
    const api = readFileSync(apiPath, 'utf8');
    expect(spark).not.toMatch(
      /export\s+(?:async\s+)?(?:function ghMove|const ghMove|class ghMove|type ghMove)\b/,
    );
    expect(spark).not.toMatch(/export\s+\{[^}]*\bghMove\b/);
    expect(spark).not.toMatch(/\bghMove\b/);
    expect(api).not.toMatch(
      /export\s+(?:async\s+)?(?:function ghMove|const ghMove|class ghMove|type ghMove)\b/,
    );
    expect(api).not.toMatch(/export\s+\{[^}]*\bghMove\b/);
    expect(api).not.toMatch(/\bghMove\b/);
  });

  it('keeps the other spark host APIs, including ghDelete', () => {
    expect(existsSync(sparkApiPath)).toBe(true);
    expect(existsSync(apiPath)).toBe(true);
    const spark = readFileSync(sparkApiPath, 'utf8');
    const api = readFileSync(apiPath, 'utf8');
    for (const name of KEPT_SPARK_EXPORTS) {
      expect(spark).toMatch(
        new RegExp(
          String.raw`export\s+(?:async\s+)?(?:function ${name}|const ${name}|class ${name}|type ${name})\b`,
        ),
      );
      expect(api).toMatch(new RegExp(String.raw`\b${name}\b`));
    }
  });
});
