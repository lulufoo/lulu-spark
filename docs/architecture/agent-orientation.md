# Agent Orientation

Structural **charter** for workbench: invariants AI and code changes must respect. Not a description of the current codebase.

Implementation details (ports, paths, APIs, modules) → code, `README.md`, operational docs.

Companion charter: [01-boundaries.md](01-boundaries.md) (write rules, planes, retrieval, change checklist).

---

## Scope

`docs/architecture/` states **non-negotiable boundaries** only. Update when an invariant below changes — not for normal feature work.

---

## System Invariants

1. **Three-store separation** — Corpus (working archive), Annotation Overlay, and Published Stores (external) stay logically distinct. No collapsing layers.
2. **Primary is append-only** — Ingest adds Primary Records; Primary body is not edited in place. Corrections are new records or overlay, not silent rewrite of history.
3. **Explicit promotion** — Published Stores receive content only through an explicit Promotion operation initiated via the Host. Ingest does not publish.
4. **Host is local authority** — Promotion, overlay mutation, and search index construction are Host responsibilities. External agents do not bypass Host for cross-store writes.
5. **Protocol adapter is a proxy** — Any MCP (or equivalent) adapter for corpus access proxies the Host's local HTTP API. It must not read the Corpus filesystem directly.
6. **Parallel retrieval** — Local full-text search, remote catalog discovery, and corpus summary read (e.g. via MCP) are separate channels. Failure of one does not redefine another.
7. **Program vs corpus split** — Host source and user corpus data live in separate repositories, bound by configuration.

---

## Glossary

| Term | Meaning |
|------|---------|
| **Program Repo** | Host application source |
| **Corpus Repo** | User working archive (Primary, Overlay, machine index, derived views) |
| **Store Repo** | External repository for published documents |
| **Primary Record** | Immutable ingested input unit |
| **Annotation Overlay** | Mutable notes and links on Primary Records |
| **Derived Layer** | Regenerable views derived from Primary (not authoritative) |
| **Machine Index** | Machine-readable entry catalog in Corpus |
| **Published Document** | Document in a Store Repo after Promotion |
| **Catalog** | Per–Store Repo discovery index |
| **Ingest** | Write Primary (+ index update); not Promotion |
| **Promotion** | Explicit Corpus → Store Repos write with traceability to Primary |
| **Host** | Desktop coordinator (Tauri app and its services) |
| **Sidecar HTTP** | Host-local HTTP API for external agents |
| **Protocol Adapter** | MCP (or similar) mapping tools → Sidecar HTTP only |
| **Search Backend** | External full-text index; optional |
| **Ingest Plane** | External agent instruction packages that ingest |
| **Host Plane** | Host + Sidecar + indexing + UI |
| **Session Guard Plane** | Hook configs constraining agent scope; no corpus semantics |

**Not the same:** Corpus Repo ≠ Store Repos; Machine Index ≠ Catalog; Derived Layer ≠ Published Document; Protocol Adapter ≠ Search Backend; Sidecar HTTP ≠ Protocol Adapter listen endpoint.

---

## Logical Topology (invariant shape only)

```
[ External input ] ── Ingest Plane ──► Corpus Repo
                         │              Primary (append-only)
                         │              Overlay (mutable)
                         │              Machine Index + Derived
                         │
                         └── Sidecar HTTP ◄── Protocol Adapter (proxy only)

Corpus Repo ── Promotion (explicit, via Host) ──► Store Repos
                                                      Published + Catalog

Host ── indexes ──► Search Backend (optional, external)
Agents ── discover Published ──► remote catalog channel (independent of MCP)
```

---

## Document Stability Policy

**Update architecture docs only when an invariant in §System Invariants changes** (new store layer, promotion rule change, adapter allowed to bypass Host, new agent plane, etc.).

Do **not** update for: new skills, API routes, ports, filenames, UI, search vendor, or module layout.

Implementation aliases (config keys, directory names) are **not normative**; see code and `README.md`.
