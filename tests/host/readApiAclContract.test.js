import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it, vi } from 'vitest'
import {
  READ_API_INVOKE_MAP,
  resolveInvokeFromPath,
} from '../../frontend/src/host/readApiInvokeMap.ts'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..')

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

  it('knowledge git read commands 已从 read-api ACL 移除', () => {
    expect(tomlAllow).not.toContain('get_kb_diff_status')
    expect(aclAllow).not.toContain('get_kb_diff_status')
    expect(tomlAllow).not.toContain('kb_status')
    expect(aclAllow).not.toContain('kb_status')
  })

  it('kb doc count command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('kb_doc_count')
    expect(aclAllow).toContain('kb_doc_count')
  })

  it('doc highlights read command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('get_doc_highlights')
    expect(aclAllow).toContain('get_doc_highlights')
  })

  it('bind session read command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('read_bind_session')
    expect(aclAllow).toContain('read_bind_session')
  })

  it('设置页不再依赖 GitHub 推断/校验命令', () => {
    for (const cmd of ['infer_github_user_url', 'check_spark_root', 'get_status']) {
      expect(tomlAllow, `${cmd} not in toml`).not.toContain(cmd)
      expect(aclAllow, `${cmd} not in acl`).not.toContain(cmd)
    }
  })

  it('READ_API_INVOKE_MAP 含 get_message_channel_unread', () => {
    expect(mappedCmds).toContain('get_message_channel_unread')
  })

  it('message channel unread command 已加入 read-api ACL', () => {
    expect(tomlAllow).toContain('get_message_channel_unread')
    expect(aclAllow).toContain('get_message_channel_unread')
  })

  it('resolveInvokeFromPath 将 snake_case channel 查询映射为 camelCase invoke 参数', () => {
    expect(
      resolveInvokeFromPath('/api/message-channel-unread?channel=notes'),
    ).toEqual({
      cmd: 'get_message_channel_unread',
      args: { channel: 'notes' },
    })
  })

  it('host/api.ts 对外暴露 getMessageChannelUnread，传输走 transport.ts', async () => {
    const api = await import('../../frontend/src/host/api.ts')
    expect(typeof api.getMessageChannelUnread).toBe('function')
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => true,
      text: async () => 'true',
    })
    await api.getMessageChannelUnread('notes')
    expect(globalThis.fetch.mock.calls[0][0]).toMatch(
      /message-channel-unread.*channel=notes/,
    )
  })

  it('页面不直接 import invoke map', () => {
    const pageFiles = [
      'frontend/src/home/page.tsx',
      'frontend/src/notes/page.tsx',
      'frontend/src/knowledge/page.tsx',
    ]
    for (const rel of pageFiles) {
      const src = readFileSync(join(repoRoot, rel), 'utf8')
      expect(src, rel).not.toMatch(/ApiInvokeMap/)
    }
  })
})
