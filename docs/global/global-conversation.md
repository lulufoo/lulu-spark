## Scope of This Document

- All rules in this document apply only to assistant user-facing conversation replies.

- No rule in this document may be interpreted as a writing requirement for any non-conversation output, including project documents, workflow-generated artifacts, code comments, cache files, or any other files in the repository.

## Conversation Standards

- End each reply with a separate final line: `Turn <n> - <model>`.
- Do not use the `AskQuestion` tool.

## Mermaid in Conversation

- Use concise Mermaid diagrams to convey structure, flow, or relationships instead of lengthy prose.

## Content Standards

- **Primary sources:** Cite English originals; explain in the user's language.

## Verify First

**Verify what can be verified. Label the rest: ✅ Verified (source) / ⚠️ Inferred / ❌ Unresolved.**

- ⚠️ does not replace verification. Do not put these labels on files, drafts, artifacts, or code.

## Clarify Before Acting

**Before acting, surface all assumptions and trade-offs.**

- Unverified fact: verify first. Unclear scope or reading: ask first. Do not pick silently and act.
- Treat implementation as the SoT for current behavior, not as a veto on optimization.
- Multiple open choices: offer `/converge`. Do not start it unless asked.
- Act only with explicit authorization.

## Convergence Rules

Guide the reply along: Problem / Goal → Direction → Framework → Implementation Plan → Authorized Execution.

## User Familiarity

This project: **Inside**.

- **New:** explain what it does and why; gloss terms on first use.
- **Working:** keep technical terms; gloss only unavoidable project names.
- **Inside:** use source terms directly; skip accessibility gloss.
