import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import {
  SYNC_API_INVOKE_MAP,
  resolveSyncInvoke,
} from '../../frontend/src/host/syncApiInvokeMap.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const relocatePath = join(repoRoot, 'src-tauri/src/services/github/relocate.rs');
const relocateUnitTestPath = join(
  repoRoot,
  'src-tauri/src/unit-tests/services/github/relocate.rs',
);
const githubModPath = join(repoRoot, 'src-tauri/src/services/github/mod.rs');
const deletePath = join(repoRoot, 'src-tauri/src/services/github/delete.rs');
const githubClientPath = join(repoRoot, 'src-tauri/src/integrations/github.rs');
const githubClientTestPath = join(
  repoRoot,
  'src-tauri/src/unit-tests/integrations/github.rs',
);
const syncCommandPath = join(repoRoot, 'src-tauri/src/commands/sync.rs');
const libPath = join(repoRoot, 'src-tauri/src/lib.rs');
const syncApiTomlPath = join(repoRoot, 'src-tauri/permissions/sync-api.toml');
const aclManifestPath = join(repoRoot, 'src-tauri/gen/schemas/acl-manifests.json');
const syncInvokeMapPath = join(repoRoot, 'frontend/src/host/syncApiInvokeMap.ts');

const GH_MOVE_FN_RE =
  /(?:pub\s+)?(?:async\s+)?(?:fn gh_move_assets|const gh_move_assets|struct gh_move_assets|type gh_move_assets)\b/;
const GH_DELETE_FN_RE =
  /(?:pub\s+)?(?:async\s+)?(?:fn gh_delete_assets|const gh_delete_assets|struct gh_delete_assets|type gh_delete_assets)\b/;
const RELOCATE_MOD_RE =
  /(?:pub\s+)?(?:mod relocate|use relocate|const relocate|type relocate)\b/;

/** @returns {string[]} */
function parseSyncApiTomlAllowCommands() {
  const toml = readFileSync(syncApiTomlPath, 'utf8');
  const block = toml.match(
    /identifier\s*=\s*"sync-api"[\s\S]*?commands\.allow\s*=\s*\[([\s\S]*?)\]/,
  );
  if (!block) throw new Error('sync-api.toml: sync-api commands.allow block not found');
  return [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
}

/** @returns {string[]} */
function parseAclManifestSyncApiAllowCommands() {
  const acl = JSON.parse(readFileSync(aclManifestPath, 'utf8'));
  const allow = acl?.['__app-acl__']?.permissions?.['sync-api']?.commands?.allow;
  if (!Array.isArray(allow)) {
    throw new Error('acl-manifests.json: sync-api commands.allow missing');
  }
  return allow;
}

/** Absolute files under `dir` whose names end with `ext`. */
function listFiles(dir, ext, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) listFiles(abs, ext, out);
    else if (name.endsWith(ext)) out.push(abs);
  }
  return out;
}

describe('relocate and gh_move_assets removed', () => {
  it('deletes relocate.rs and the GitHub services module', () => {
    expect(existsSync(relocatePath)).toBe(false);
    expect(existsSync(githubModPath)).toBe(false);
    expect(existsSync(deletePath)).toBe(false);
  });

  it('removes the gh_move_assets command and its lib.rs registration', () => {
    expect(existsSync(syncCommandPath)).toBe(true);
    expect(existsSync(libPath)).toBe(true);
    const syncRs = readFileSync(syncCommandPath, 'utf8');
    const libRs = readFileSync(libPath, 'utf8');
    expect(syncRs).not.toMatch(GH_MOVE_FN_RE);
    expect(syncRs).not.toMatch(/\bgh_move_assets\b/);
    expect(libRs).not.toMatch(/commands::sync::gh_move_assets\b/);
    expect(libRs).not.toMatch(/\bgh_move_assets\b/);
  });

  it('drops gh_move_assets from sync-api.toml and acl-manifests.json allow lists', () => {
    const tomlAllow = parseSyncApiTomlAllowCommands();
    const aclAllow = parseAclManifestSyncApiAllowCommands();
    expect(tomlAllow).not.toContain('gh_move_assets');
    expect(aclAllow).not.toContain('gh_move_assets');
    expect(readFileSync(syncApiTomlPath, 'utf8')).not.toMatch(/\bgh_move_assets\b/);
    expect(readFileSync(aclManifestPath, 'utf8')).not.toMatch(/\bgh_move_assets\b/);
  });

  it('does not map /api/gh-move to gh_move_assets', () => {
    expect(SYNC_API_INVOKE_MAP['/api/gh-move']).toBeUndefined();
    expect(resolveSyncInvoke('/api/gh-move', { src_url: 'a', dst_dir_url: 'b' })).toBeNull();
    const invokeMap = readFileSync(syncInvokeMapPath, 'utf8');
    expect(invokeMap).not.toMatch(/['"]\/api\/gh-move['"]/);
    expect(invokeMap).not.toMatch(/\bgh_move_assets\b/);
  });

  it('removes gh_delete_assets, /api/gh-delete, delete.rs, and the GitHub client', () => {
    expect(existsSync(deletePath)).toBe(false);
    expect(existsSync(githubClientPath)).toBe(false);
    expect(existsSync(syncCommandPath)).toBe(true);
    const syncRs = readFileSync(syncCommandPath, 'utf8');
    const libRs = readFileSync(libPath, 'utf8');
    expect(syncRs).not.toMatch(GH_DELETE_FN_RE);
    expect(syncRs).not.toMatch(/\bgh_delete_assets\b/);
    expect(libRs).not.toMatch(/commands::sync::gh_delete_assets\b/);
    expect(parseSyncApiTomlAllowCommands()).not.toContain('gh_delete_assets');
    expect(parseAclManifestSyncApiAllowCommands()).not.toContain('gh_delete_assets');
    expect(SYNC_API_INVOKE_MAP['/api/gh-delete']).toBeUndefined();
    expect(
      resolveSyncInvoke('/api/gh-delete', { url: 'https://github.com/o/r/blob/main/a.md' }),
    ).toBeNull();
  });

  it('deletes relocate.rs unit tests instead of keeping gh_move_assets tests elsewhere', () => {
    expect(existsSync(relocateUnitTestPath)).toBe(false);
    const leftover = listFiles(join(repoRoot, 'src-tauri/src'), '.rs')
      .filter((abs) => {
        const text = readFileSync(abs, 'utf8');
        return GH_MOVE_FN_RE.test(text) || /\bgh_move_assets\b/.test(text);
      })
      .map((abs) => abs.slice(repoRoot.length + 1));
    expect(leftover).toEqual([]);
  });

  it('deletes GitHub client lib tests', () => {
    expect(existsSync(githubClientTestPath)).toBe(false);
  });
});
