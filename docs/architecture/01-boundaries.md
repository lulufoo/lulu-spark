# Boundaries

Structural **charter** (part 2): write rules, agent planes, retrieval independence, PR checklist. Not a description of the current codebase.

No API paths, ports, or filenames.

Companion charter: [agent-orientation.md](agent-orientation.md) (invariants, glossary, topology, stability policy).

---

## Storage Layers (Corpus Repo)

| Layer | Mutability | Who may write |
|-------|------------|---------------|
| Primary | Append-only | Ingest Plane (via Host or configured corpus access); Host admin |
| Annotation Overlay | Mutable | Host only |
| Machine Index | Updated with ingest | Ingest (with Primary); Host |
| Derived Layer | Regenerable | Host only (not Ingest direct) |

Store Repos:

| Layer | Mutability | Who may write |
|-------|------------|---------------|
| Published Document | Via Promotion | Host only |
| Catalog | Via Promotion / rebuild | Host only |

Ingest Plane must not write Store Repos directly.

---

## Promotion Boundary

1. **Gate** — Promotion is separate from Ingest. Completing ingest or derived generation does not publish.
2. **Traceability** — Published Documents link to contributing Primary Record(s). Overlay may link to Published URLs. M:N allowed.
3. **Side effects** — Host updates Search Backend indexes after Promotion when search is enabled.

Whether content is *ready* for Promotion is outside this charter.

---

## Agent Planes

| Plane | May | Must not |
|-------|-----|----------|
| **Ingest** | Write Primary + Machine Index | Write Overlay, Store Repos, or bypass Host for cross-store ops |
| **Host** | All corpus layers, Promotion, indexing, UI session | — |
| **Session Guard** | Constrain agent read/write scope | Define or alter corpus semantics |
| **Protocol Adapter** | Proxy corpus ops to Sidecar HTTP | Read Corpus filesystem directly |

Ingest may use Sidecar HTTP (Host running) or direct filesystem against configured corpus root — same invariants, different deployment channel.

---

## Retrieval Channels (independent)

| Need | Channel | Not a substitute for |
|------|---------|------------------------|
| Corpus derived/summary text in agent context | Protocol Adapter → Sidecar HTTP | Remote catalog |
| Published Store content | Remote catalog (via Host integrations) | MCP corpus tools |
| Local full-text search | Search Backend (via Host) | Either above |

Channels fail independently; do not merge semantics on failure.

---

## Change Checklist

Before a PR that touches Host, MCP, ingest, or corpus/store layout, verify:

- [ ] Primary append-only preserved
- [ ] No ingest → Store Repos shortcut
- [ ] Protocol Adapter still proxies Sidecar HTTP only
- [ ] Corpus Repo and Store Repos remain separate roots
- [ ] New retrieval path documented outside architecture docs unless an invariant changed

If an invariant must change, update both architecture charter files, then implement.
