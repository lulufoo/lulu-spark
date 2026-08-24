## Scope of This Document

This document constrains conversation replies only, not non-conversation output, except the Superpowers ban which also governs skill selection and actions.

## Conversation Standards

- End each reply with a separate final line: `Turn <n> - <model>`.
- Do not use the `AskQuestion` tool.
- Superpowers is forbidden: do not invoke, announce, or follow any Superpowers plugin skill (`.cursor/plugins/local/superpowers/`), including `using-superpowers` and `finishing-a-development-branch`.

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

HARD-GATE: Replies in this project must be followable by **New**.

- **New:** a first-time project inheritor with no prior context can follow.
- **Oriented:** a reader who knows the purpose, not the structure, can place this point in the whole.
- **Familiar:** a reader who knows the core concepts and workflows can proceed without a recap.
- **Fluent:** a peer who shares the project's language can act on this point.
