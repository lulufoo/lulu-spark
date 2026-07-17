#!/usr/bin/env python3
"""Clean Cursor agent-transcript messages into workbench-like dialogue raw.

Design: Cursor chrome (timestamp / user_query wrappers / attached skill dumps)
is NOT user speech. Extract the utterance, keep skill names as metadata only.
"""
from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

USER_QUERY_RE = re.compile(r"<user_query>\s*(.*?)\s*</user_query>", re.S | re.I)
TIMESTAMP_RE = re.compile(r"<timestamp>\s*(.*?)\s*</timestamp>", re.S | re.I)
ATTACHED_SKILLS_RE = re.compile(
    r"<manually_attached_skills>\s*(.*?)\s*</manually_attached_skills>", re.S | re.I
)
# Other Cursor chrome blocks to strip when no user_query wrapper
CHROME_BLOCKS = [
    re.compile(rf"<{name}>\s*.*?\s*</{name}>", re.S | re.I)
    for name in (
        "open_and_recently_viewed_files",
        "agent_skills",
        "user_info",
        "user_rules",
        "rules",
        "mcp_file_system",
        "system_reminder",
        "communication",
        "citing_code",
        "terminal_files_information",
        "always_applied_workspace_rules",
        "attached_files",
        "code_selection",
        "agent_transcripts",
        "functions",
        "mcp_meta_tools",
    )
]


def _content_to_text(content: Any) -> str:
    """Join all text parts (used for User messages)."""
    if content is None:
        return ""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts: list[str] = []
        for part in content:
            if isinstance(part, dict) and part.get("type") == "text":
                parts.append(part.get("text") or "")
            elif isinstance(part, str):
                parts.append(part)
        return "\n".join(parts)
    return str(content)


def _is_tool_preamble(text: str) -> bool:
    """Staging / English process narration before tool calls.

    Keep substantive Chinese (or balanced) deliverables even if tools follow.
    """
    s = (text or "").strip()
    if not s:
        return True
    zh = len(re.findall(r"[\u4e00-\u9fff]", s))
    en = len(re.findall(r"[A-Za-z]", s))
    # Substantive Chinese deliverable — keep even when English identifiers
    # dominate letter counts (paths, APIs, SKILL names).
    if zh >= 40:
        return False
    if en >= 30 and en > zh:
        return True
    if len(s) < 160:
        return True
    return en > zh


def assistant_text_from_content(content: Any) -> str:
    """Extract assistant prose for clean-raw.

    Mechanical rules:
    1. Skip non-text parts (tool_use, etc.).
    2. Text before/between tool_use in the same message is dropped when it
       looks like a tool preamble / process narration.
    3. Text-only messages and post-tool text are kept.
    """
    if content is None:
        return ""
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return str(content)

    tool_idxs = [
        i
        for i, part in enumerate(content)
        if isinstance(part, dict) and part.get("type") == "tool_use"
    ]
    first_tool = tool_idxs[0] if tool_idxs else None
    last_tool = tool_idxs[-1] if tool_idxs else None

    parts: list[str] = []
    for i, part in enumerate(content):
        if isinstance(part, str):
            text = part
        elif isinstance(part, dict) and part.get("type") == "text":
            text = part.get("text") or ""
        else:
            continue
        if not str(text).strip():
            continue
        if first_tool is not None and i < first_tool and _is_tool_preamble(text):
            continue
        if (
            last_tool is not None
            and first_tool is not None
            and first_tool <= i < last_tool
            and _is_tool_preamble(text)
        ):
            continue
        parts.append(str(text))
    return "\n".join(parts)



_MONTH = {
    "jan": 1, "feb": 2, "mar": 3, "apr": 4, "may": 5, "jun": 6,
    "jul": 7, "aug": 8, "sep": 9, "oct": 10, "nov": 11, "dec": 12,
}
# "Tuesday, Jul 14, 2026, 6:57 PM (UTC+8)"
_TS_HUMAN = re.compile(
    r"(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday),\s+"
    r"(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+"
    r"(\d{1,2}),\s+(\d{4}),\s+"
    r"(\d{1,2}):(\d{2})\s*(AM|PM)\s*"
    r"\(UTC([+-]\d{1,2})\)",
    re.I,
)


def to_iso_ts(value: str | None) -> str | None:
    """Normalize Cursor/human time to ISO-8601, e.g. 2026-07-14T18:57:00+08:00."""
    if value is None:
        return None
    s = str(value).strip()
    if not s:
        return None
    if re.fullmatch(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?", s
    ):
        if s.endswith("Z"):
            return s[:-1] + "+00:00"
        if re.search(r"[+-]\d{2}$", s):
            return s + ":00"
        return s
    if re.fullmatch(r"\d{10}", s):
        return datetime.fromtimestamp(int(s), tz=timezone.utc).isoformat()
    if re.fullmatch(r"\d{13}", s):
        return datetime.fromtimestamp(int(s) / 1000, tz=timezone.utc).isoformat()
    m = _TS_HUMAN.search(s)
    if not m:
        return s
    mon = _MONTH[m.group(1).lower()[:3]]
    day = int(m.group(2))
    year = int(m.group(3))
    hour = int(m.group(4))
    minute = int(m.group(5))
    ampm = m.group(6).upper()
    if ampm == "PM" and hour != 12:
        hour += 12
    if ampm == "AM" and hour == 12:
        hour = 0
    off = int(m.group(7))
    sign = "+" if off >= 0 else "-"
    return (
        f"{year:04d}-{mon:02d}-{day:02d}T{hour:02d}:{minute:02d}:00"
        f"{sign}{abs(off):02d}:00"
    )


def clean_user_text(raw: str) -> dict[str, Any]:
    """Return {text, timestamp} from a Cursor user message blob. Skills stripped, not kept."""
    timestamp = None
    m_ts = TIMESTAMP_RE.search(raw)
    if m_ts:
        timestamp = to_iso_ts(m_ts.group(1).strip())

    m_q = USER_QUERY_RE.search(raw)
    if m_q:
        text = m_q.group(1).strip()
    else:
        text = raw
        text = ATTACHED_SKILLS_RE.sub("", text)
        text = TIMESTAMP_RE.sub("", text)
        for cre in CHROME_BLOCKS:
            text = cre.sub("", text)
        text = text.strip()

    # collapse excessive blank lines introduced by stripping
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    return {"text": text, "timestamp": timestamp}


_PROCESS_LINE = re.compile(
    r"^(?:Let me |I(?:'m| am| need| will|'ll| should| can|'ve) |"
    r"Now I |There's |There is |Looking |Checking |Reading |Exploring |"
    r"I'll (?:read|check|search|verify|look)|"
    r"I will (?:read|check|search|verify|look)|"
    r"正在|接下来(?=做|查|读|跑)|我来(?=查|读|搜))",
    re.I,
)

# English thinking / orchestration openers (paragraph-level drop)
_EN_THINK_START = re.compile(
    r"^(?:"
    r"I(?:'m| am| need| will|'ll| should| can|'ve| found| see| think| notice| want) |"
    r"Let me |Looking |Checking |Reading |Exploring |Wait,|Okay,|Hmm,|Actually,|"
    r"The (?:user|second|third|first|key|core|real|current|existing|drafting|eval|unresolved) |"
    r"This (?:means|gets|is|approval) |So (?:this|the|I|both) |"
    r"Now (?:I|I'?m|the) |There(?:'s| is) |Based on |Given (?:that|the) |"
    r"My response |Before I |However,|For (?:rollout|precondition|coverage) |"
    r"Second,|Third,|First,|Also,|On the user|"
    r"Direction |Option \d|INV:|"
    r"to the \d)"
    ,
    re.I,
)


def _zh_en_counts(s: str) -> tuple[int, int]:
    return (
        len(re.findall(r"[\u4e00-\u9fff]", s)),
        len(re.findall(r"[A-Za-z]", s)),
    )


def _is_english_process_para(s: str) -> bool:
    """True if a paragraph is English thinking / process narration to drop."""
    s = (s or "").strip()
    if not s:
        return False
    # Keep fenced code / tables-ish blocks that are mostly structural
    if s.startswith("```"):
        return False
    zh, en = _zh_en_counts(s)
    # English-led thinking: judge by the opening window even if Chinese terms appear later
    if _EN_THINK_START.match(s):
        head = s[:160]
        hzh, hen = _zh_en_counts(head)
        if hen >= 28 and hen > hzh * 1.5:
            return True
        if en > zh and hen >= 24:
            return True
    # Substantive Chinese deliverable
    if zh >= 40:
        return False
    # Chinese-led tech lines (identifiers inflate `en`) — keep
    if zh >= 8 and zh * 6 >= en:
        return False
    if en < 28:
        return False
    if zh <= 4 and en >= 28:
        return True
    if zh <= 10 and en >= 80 and en > zh * 3:
        return True
    return False


def _drop_prev_turn_echo(text: str, prev: str) -> str:
    """Drop paragraphs that are echoes of the previous turn's assistant body.

    Cursor transcripts sometimes re-emit the prior TurnN deliverable into the
    next user turn's assistant window; strip those echoes for clean-raw.
    """
    text = (text or "").strip()
    prev = (prev or "").strip()
    if not text or not prev or len(prev) < 80:
        return text
    kept: list[str] = []
    for para in re.split(r"\n\n+", text):
        p = para.strip()
        if not p:
            continue
        if len(p) >= 80 and p in prev:
            continue
        if len(p) >= 80 and p[:120] in prev:
            continue
        # Bare TurnN label or TurnN block already present in prev
        if re.fullmatch(r"Turn\d+", p):
            continue
        if re.match(r"^Turn\d+\b", p) and (p in prev or p[:200] in prev):
            continue
        # Heading echo from previous turn (skeleton left after body paras dropped)
        first_line = p.split("\n", 1)[0].strip()
        if first_line.startswith("## ") and first_line in prev:
            continue
        if len(p) >= 8 and p in prev:
            continue
        kept.append(para)
    return "\n\n".join(kept).strip()


def _unglue_english_tail(para: str) -> str:
    """Cut English thinking glued onto a Chinese sentence in the same paragraph.

    Common Cursor leak: 「……堵住。 is a bigger architectural shift. Direction B…」
    — no blank line, so paragraph-level filters keep the whole block via zh≥40.
    """
    para = para or ""
    if not para.strip():
        return para
    # Also split before mid-paragraph Direction / But there's… after Chinese text
    cut = None
    for m in re.finditer(r"[。！？]", para):
        rest = para[m.end() :].lstrip()
        if not rest:
            continue
        zh, en = _zh_en_counts(rest)
        starts_latin = bool(re.match(r"^[A-Za-z]", rest))
        # leftover mid-sentence English ("is a bigger…") or Direction-style blocks
        if starts_latin and en >= 40 and en > max(zh, 1) * 2:
            cut = m.end()
            break
        if starts_latin and _EN_THINK_START.match(rest) and en >= 28:
            cut = m.end()
            break
    if cut is None:
        # Fallback: Chinese present, then long English run starting mid-para
        m2 = re.search(
            r"([\u4e00-\u9fff][^A-Za-z]{0,40})"
            r"(\s+(?:is a |Direction |But there(?:'s| is)|If [A-Z]|The (?:user|second|key|core)|"
            r"I(?:'m| am| need)|Looking |However,|So (?:this|the|I)|This (?:means|is)))",
            para,
        )
        if m2:
            rest = para[m2.start(2) :].lstrip()
            zh, en = _zh_en_counts(rest)
            if en >= 40 and en > max(zh, 1) * 2:
                cut = m2.start(2)
    if cut is None:
        return para.strip()
    return para[:cut].strip()


def _strip_english_process_paragraphs(text: str) -> str:
    """Drop English thinking paragraphs; cut trailing English after Chinese body."""
    paras = re.split(r"\n\n+", text)
    kept: list[str] = []
    last_zh_keep = -1
    for para in paras:
        para = _unglue_english_tail(para)
        if not para.strip():
            continue
        if _is_english_process_para(para):
            continue
        kept.append(para)
        zh, _en = _zh_en_counts(para)
        if zh >= 8:
            last_zh_keep = len(kept) - 1
    if not kept:
        return ""
    # If we had Chinese substance, drop any trailing English-dominant leftovers
    if last_zh_keep >= 0 and last_zh_keep < len(kept) - 1:
        trailing = kept[last_zh_keep + 1 :]
        # keep trailing only if it still has Chinese substance
        extra: list[str] = []
        for para in trailing:
            zh, en = _zh_en_counts(para)
            if zh >= 8 and not _is_english_process_para(para):
                extra.append(para)
            elif zh >= 8:
                extra.append(para)
            # else drop English-only trailing
        kept = kept[: last_zh_keep + 1] + extra
    return "\n\n".join(kept).strip()


def clean_assistant_text(raw: str) -> str:
    """Normalize assistant prose after assistant_text_from_content filtering.

    Drops short process one-liners and English thinking / interrupted
    orchestration paragraphs (including trailing English after Chinese body).
    """
    text = (raw or "").strip()
    if not text:
        return ""
    out: list[str] = []
    in_fence = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("```"):
            in_fence = not in_fence
            out.append(line)
            continue
        if in_fence:
            out.append(line)
            continue
        if not stripped:
            out.append(line)
            continue
        zh, en = _zh_en_counts(stripped)
        if (
            _PROCESS_LINE.match(stripped)
            and en >= 20
            and en > zh * 2
            and len(stripped) < 240
        ):
            continue
        # Drop long English-only lines outside fences
        if zh == 0 and en >= 40 and _EN_THINK_START.match(stripped):
            continue
        out.append(line)
    text = "\n".join(out)
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    text = _strip_english_process_paragraphs(text)
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    return text


def turns_from_jsonl(path: Path) -> list[dict[str, Any]]:
    turns: list[dict[str, Any]] = []
    user_i = 0
    with path.open(encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            role = obj.get("role")
            msg = obj.get("message") or {}
            content = msg.get("content") if isinstance(msg, dict) else None
            if role == "user":
                raw_text = _content_to_text(content)
                if not raw_text.strip():
                    continue
                cleaned = clean_user_text(raw_text)
                if not cleaned["text"]:
                    continue
                user_i += 1
                turns.append(
                    {
                        "role": "user",
                        "turn": user_i,
                        "text": cleaned["text"],
                        "timestamp": cleaned["timestamp"],
                    }
                )
            elif role == "assistant":
                raw_text = assistant_text_from_content(content)
                text = clean_assistant_text(raw_text)
                if not text:
                    continue
                turns.append({"role": "assistant", "turn": None, "text": text})
    return turns



def renumber_compact_turns(paired: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Force clean-raw turn ids to contiguous 1..N (SSOT for distill)."""
    out: list[dict[str, Any]] = []
    for i, t in enumerate(paired, start=1):
        turn = dict(t)
        turn["n"] = i
        turn.pop("turn_id", None)
        out.append(turn)
    return out


def to_compact_turns(paired: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Normalize any paired shape to compact {n,u,a,ts?} then renumber 1..N."""
    out: list[dict[str, Any]] = []
    for t in paired:
        if "n" in t and "u" in t:
            turn: dict[str, Any] = {
                "n": int(t.get("n") or 0),
                "u": t.get("u") or "",
                "a": t.get("a") or "",
            }
            ts = to_iso_ts(t.get("ts"))
            if ts:
                turn["ts"] = ts
            out.append(turn)
            continue
        user = t.get("user") or {}
        ai = t.get("ai") or {}
        turn = {
            "n": int(t.get("turn_id") or t.get("n") or 0),
            "u": user.get("text") or t.get("u") or "",
            "a": ai.get("text_raw") or ai.get("text") or t.get("a") or "",
        }
        ts = to_iso_ts(user.get("timestamp") or t.get("ts"))
        if ts:
            turn["ts"] = ts
        out.append(turn)
    return renumber_compact_turns(out)


def _join_assistant_chunks(chunks: list[str]) -> str:
    """Join per-message assistant texts, then re-strip English process residue."""
    parts = [c.strip() for c in chunks if (c or "").strip()]
    if not parts:
        return ""
    return clean_assistant_text("\n\n".join(parts))


def pair_flat_turns(flat: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Pair flat user/assistant rows into compact clean-raw turns."""
    paired: list[dict[str, Any]] = []
    pending_user: dict[str, Any] | None = None
    ai_buf: list[str] = []
    prev_a = ""
    for item in flat:
        role = item.get("role")
        if role == "user":
            if pending_user is not None:
                a = _drop_prev_turn_echo(_join_assistant_chunks(ai_buf), prev_a)
                turn: dict[str, Any] = {
                    "n": int(pending_user["turn"]),
                    "u": pending_user["text"],
                    "a": a,
                }
                ts = to_iso_ts(pending_user.get("timestamp"))
                if ts:
                    turn["ts"] = ts
                paired.append(turn)
                prev_a = a
                ai_buf = []
            pending_user = item
        elif role in ("assistant", "ai"):
            ai_buf.append(item.get("text") or "")
    if pending_user is not None:
        a = _drop_prev_turn_echo(_join_assistant_chunks(ai_buf), prev_a)
        turn = {
            "n": int(pending_user["turn"]),
            "u": pending_user["text"],
            "a": a,
        }
        ts = to_iso_ts(pending_user.get("timestamp"))
        if ts:
            turn["ts"] = ts
        paired.append(turn)
    return renumber_compact_turns(paired)


def build_clean_raw_doc(
    session_id: str,
    paired: list[dict[str, Any]],
    *,
    source: str,
    title: str | None = None,
    cleaned_by: str = "transcript-clean v1",  # ignored; kept for call-compat
) -> dict[str, Any]:
    del cleaned_by  # not persisted
    return {
        "sid": session_id,
        "title": title or session_id,
        "src": source,
        "turns": to_compact_turns(paired),
    }


def write_clean_raw_json(path: Path, doc: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return path


def assert_no_chrome_text(text: str) -> list[str]:
    bad = []
    for tag in ("user_query", "timestamp", "manually_attached_skills"):
        if f"<{tag}>" in text:
            bad.append(tag)
    return bad


def assert_no_chrome_doc(doc: dict[str, Any]) -> list[str]:
    """Chrome must not remain in User utterances."""
    bad: list[str] = []
    for t in doc.get("turns") or []:
        user = t.get("u") if isinstance(t.get("u"), str) else ((t.get("user") or {}).get("text") or "")
        hits = assert_no_chrome_text(user or "")
        if hits:
            bad.append(f"turn {t.get('n', t.get('turn_id'))}: {','.join(hits)}")
    return bad


def normalize_clean_turns(doc: dict[str, Any]) -> list[dict[str, Any]]:
    """Return internal paired turns for distill-state init.

    Accepts compact clean-raw (`n/u/a/ts`) and legacy (`turn_id/user/ai`).
    """
    out: list[dict[str, Any]] = []
    for t in doc.get("turns") or []:
        if "n" in t and "u" in t:
            out.append(
                {
                    "turn_id": int(t["n"]),
                    "user": {
                        "text": t.get("u") or "",
                        "timestamp": to_iso_ts(t.get("ts")),
                        "attached_skills": [],
                    },
                    "ai": {"text_raw": t.get("a") or ""},
                }
            )
            continue
        # legacy
        user = t.get("user") or {}
        ai = t.get("ai") or {}
        out.append(
            {
                "turn_id": int(t.get("turn_id") or 0),
                "user": {
                    "text": user.get("text") or "",
                    "timestamp": to_iso_ts(user.get("timestamp")),
                    "attached_skills": [],
                },
                "ai": {"text_raw": ai.get("text_raw") or ai.get("text") or ""},
            }
        )
    for i, row in enumerate(out, start=1):
        row["turn_id"] = i
    return out
