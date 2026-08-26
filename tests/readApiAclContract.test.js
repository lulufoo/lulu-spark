import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { READ_API_INVOKE_MAP } from '../frontend/js/readApiInvokeMap.js'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..')

/** @returns {string[]} */
function parseReadApiTomlAllowCommands() {
  const toml = readFileSync(
    join(repoRoot, 'src-tauri/permissions/read-api.toml'),
    'utf8',
  )
  const block = toml.match(
    /identifier\s*=\s*"read-api"[\s\S]*?commands\.allow\s*=\s*\[([\s\S]*?)\]/,
  )
  if (!block) throw new Error('read-api.toml: read-api commands.allow block not found')
  return [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1])
}

/** @returns {string[]} */
function parseAclManifestReadApiAllowCommands() {
  const acl = JSON.parse(
    readFileSync(join(repoRoot, 'src-tauri/gen/schemas/acl-manifests.json'), 'utf8'),
  )
  const allow = acl?.['__app-acl__']?.permissions?.['read-api']?.commands?.allow
  if (!Array.isArray(allow)) {
    throw new Error('acl-manifests.json: read-api commands.allow missing')
  }
  return allow
}

/** 前端 READ_API_INVOKE_MAP 里出现的 Tauri 命令名 */
function readApiInvokeMapCommands() {
  return [...new Set(Object.values(READ_API_INVOKE_MAP).map((e) => e.cmd))].sort()
}

describe('read-api ACL 与前端 invoke 映射一致', () => {
  const tomlAllow = parseReadApiTomlAllowCommands()
  const aclAllow = parseAclManifestReadApiAllowCommands()
  const mappedCmds = readApiInvokeMapCommands()

  it('read-api.toml 与 acl-manifests.json 的 allow 列表相同', () => {
    expect([...tomlAllow].sort()).toEqual([...aclAllow].sort())
  })

  it('READ_API_INVOKE_MAP 中每个 cmd 均在 read-api.toml ACL 中（防止 infer 等漏配）', () => {
    const missing = mappedCmds.filter((cmd) => !tomlAllow.includes(cmd))
    expect(missing, `missing in read-api.toml: ${missing.join(', ')}`).toEqual([])
  })

  it('READ_API_INVOKE_MAP 中每个 cmd 均在 acl-manifests.json 中', () => {
    const missing = mappedCmds.filter((cmd) => !aclAllow.includes(cmd))
    expect(missing, `missing in acl-manifests: ${missing.join(', ')}`).toEqual([])
  })

  it('kb diff status command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('get_kb_diff_status')
    expect(aclAllow).toContain('get_kb_diff_status')
  })

  it('kb doc count command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('kb_doc_count')
    expect(aclAllow).toContain('kb_doc_count')
  })

  it('bind session read command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('read_bind_session')
    expect(aclAllow).toContain('read_bind_session')
  })

  it('设置页依赖的推断/校验命令已列入 ACL', () => {
    for (const cmd of ['infer_github_user_url', 'check_workbench_knowledge_root']) {
      expect(tomlAllow, `${cmd} in toml`).toContain(cmd)
      expect(aclAllow, `${cmd} in acl`).toContain(cmd)
    }
  })
})
