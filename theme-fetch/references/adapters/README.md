# Platform Adapters — Phase 0 / Phase 1

> Phase 0 Resolve 输出 `platform` 字符串；Phase 1 Acquire 读取对应 adapter 文档，产出 [ArticleBundle](../bundle-schema.md)。

---

## Phase 0 routing table

| Input pattern | `platform` | Adapter doc | Status |
|---------------|-------------|-------------|--------|
| `mp.weixin.qq.com/s/*` | `wechat` | [wechat.md](wechat.md) | **implemented** |
| `zhuanlan.zhihu.com/p/*` | `zhihu` | zhihu.md | planned |
| `juejin.cn/post/*` | `juejin` | juejin.md | planned |
| `www.infoq.cn/article/*` (full article text) | `infoq-article` | infoq-article.md | planned |
| Local `.html`, pasted HTML, unknown URL | `plain-html` | [plain-html.md](plain-html.md) | **implemented** (basic) |

**Cross-skill routing:**

| URL | Skill |
|-----|-------|
| `youtube.com/*`, `youtu.be/*` | theme-line |
| `infoq.cn/video/*` | theme-line |
| `mp.weixin.qq.com/s/*` | **theme-fetch** |

未知 URL → `platform=plain-html`；提示用户可粘贴 HTML 或本地文件。

---

## Adapter doc structure

每个 adapter 文档 MUST 含四段：

1. **Match** — URL/输入如何识别
2. **Acquire** — 如何拉取原始数据（优先 `scripts/`）
3. **Map** — 如何映射为 ArticleBundle
4. **Quirks** — 降级与边界情况

---

## New adapter checklist

- [ ] Match 规则不与现有 adapter 重叠
- [ ] Acquire 步骤可复现（脚本或完整 curl）
- [ ] Map 输出 conform [bundle-schema.md](../bundle-schema.md)
- [ ] `source.adapter` 设为 `{platform}@v1`
- [ ] Quirks 文档化所有降级路径
- [ ] 更新本 README 路由表
- [ ] 更新 [SKILL.md](../../SKILL.md) Phase 0 路由表（保持与本表一致）

---

## Versioning

- Adapter 版本字符串：`{platform}@v1`
- Bundle 契约版本：`schema_version: 1`（见 bundle-schema）
