#!/usr/bin/env python3
"""
DDM Normalize — 将 raw/ 目录下未格式化的对话文档转换为 DDM 标准格式。

判断是否已格式化：文件名以 -normalized.md 结尾则跳过。
输出：<原文件名>-normalized.md，写入同一 raw/ 目录。

环境变量：
  COGNITIVE_TRACE_ARCHIVE_ROOT  覆盖默认归档根路径（默认 ~/Code/cognitive-trace-archive）
"""

import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

# ---------------------------------------------------------------------------
# 路径配置
# ---------------------------------------------------------------------------

ARCHIVE_ROOT = Path(
    os.environ.get("COGNITIVE_TRACE_ARCHIVE_ROOT", Path.home() / "Code" / "cognitive-trace-archive")
)
RAW_DIR    = ARCHIVE_ROOT / "raw"
INDEX_PATH = ARCHIVE_ROOT / "index.json"

# ---------------------------------------------------------------------------
# 数据模型
# ---------------------------------------------------------------------------

@dataclass
class Turn:
    role: str          # "user" | "ai"
    todo_n: Optional[int]  # AI 轮次编号，User 为 None
    content: str       # 正文（已去除前后空白行）


@dataclass
class Dialogue:
    title: str
    turns: list[Turn] = field(default_factory=list)


# ---------------------------------------------------------------------------
# 输入解析（待实现）
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# Cursor 导出格式解析
# ---------------------------------------------------------------------------

# AI 说话人名称（Cursor 导出可能出现的变体）
_CURSOR_AI_NAMES = re.compile(
    r"^\*\*(?:Cursor|Copilot|AI|Assistant|Claude|GPT)\*\*\s*$"
)
_CURSOR_USER = re.compile(r"^\*\*User\*\*\s*$")

# 需要从 AI 正文末尾剥离的元数据行模式
_META_LINE_PATTERNS = [
    re.compile(r"^CHAT_ID-[A-Za-z0-9\-]+$"),                 # CHAT_ID-xxx
    re.compile(r"^TODO:\d*(,\d+)*$"),                         # TODO:1,2,3 / TODO:
    re.compile(r"^TODO:\s*$"),                                # TODO: （空）
    re.compile(r"^我是\s.{0,60}模型"),                         # 模型签名
    re.compile(r"^_Exported on "),                            # 导出时间戳行
]


def _is_meta_line(line: str) -> bool:
    s = line.strip()
    return any(p.search(s) for p in _META_LINE_PATTERNS)


def _strip_ai_metadata(text: str) -> str:
    """从 AI 正文尾部逐行向上剥离元数据，直到遇到实质内容为止。"""
    lines = text.split("\n")
    # 从末尾向前找最后一行非元数据、非空行
    end = len(lines)
    while end > 0:
        line = lines[end - 1].strip()
        if line == "" or _is_meta_line(line):
            end -= 1
        else:
            break
    return "\n".join(lines[:end]).strip()


def _extract_todo_n(text: str) -> tuple[Optional[int], str]:
    """
    从文本开头提取 TODO N（如 'TODO1 正文' 或 'TODO1\n正文'），
    返回 (编号, 去掉前缀后的正文)。未匹配时返回 (None, 原文)。
    """
    m = re.match(r"^TODO(\d+)\s*", text, re.IGNORECASE)
    if m:
        return int(m.group(1)), text[m.end():].strip()
    return None, text


def _strip_leading_metadata(text: str) -> str:
    """剥离正文开头的元数据行（CHAT_ID 等），直到遇到实质内容为止。"""
    lines = text.split("\n")
    start = 0
    while start < len(lines):
        line = lines[start].strip()
        if line == "" or _is_meta_line(line):
            start += 1
        else:
            break
    return "\n".join(lines[start:]).strip()


def _parse_cursor(content: str, filename_stem: str) -> Dialogue:
    """
    解析 Cursor 导出 Markdown 格式：
      # Title
      _Exported on ..._
      ---
      **User**
      <正文>
      ---
      **Cursor**
      TODO N <正文>
      ...元数据...
      ---
    """
    # 提取标题：第一个 # 标题行
    title = filename_stem
    for line in content.splitlines():
        m = re.match(r"^#\s+(.+)$", line)
        if m:
            title = m.group(1).strip()
            break

    # 按 \n---\n 切分块（兼容行首/行尾变体）
    raw_blocks = re.split(r"\n\s*---\s*\n", content)

    turns: list[Turn] = []
    ai_seq = 0  # 用于无 TODO N 时自动编号

    for block in raw_blocks:
        block = block.strip()
        if not block:
            continue

        lines = block.splitlines()
        first_line = lines[0].strip() if lines else ""

        if _CURSOR_USER.match(first_line):
            body = "\n".join(lines[1:]).strip()
            if body:
                turns.append(Turn(role="user", todo_n=None, content=body))

        elif _CURSOR_AI_NAMES.match(first_line):
            body = "\n".join(lines[1:]).strip()
            body = _strip_ai_metadata(body)
            todo_n, body = _extract_todo_n(body)
            body = _strip_leading_metadata(body)
            if todo_n is None:
                ai_seq += 1
                todo_n = ai_seq
            else:
                ai_seq = todo_n
            turns.append(Turn(role="ai", todo_n=todo_n, content=body))

    return Dialogue(title=title, turns=turns)


# ---------------------------------------------------------------------------
# GitHub Copilot Chat 导出格式解析
# ---------------------------------------------------------------------------

# 标题行：# Copilot Chat Conversation Export: <Title>
_COPILOT_TITLE_RE = re.compile(
    r"^#\s+Copilot Chat Conversation Export:\s+(.+)$", re.MULTILINE
)
# 说话人 H2：## @username（用户）或 ## Copilot（AI）
_COPILOT_TURN_RE = re.compile(
    r"^## (@\S+|Copilot)\s*$", re.MULTILINE
)

# Copilot 元数据尾行（在 _META_LINE_PATTERNS 基础上扩展）
_COPILOT_META_PATTERNS = _META_LINE_PATTERNS + [
    re.compile(r"^我是基于.{0,80}模型"),   # 模型签名变体
    re.compile(r"^我是\s*GitHub Copilot"),
]


def _is_copilot_meta_line(line: str) -> bool:
    s = line.strip()
    return any(p.search(s) for p in _COPILOT_META_PATTERNS)


def _strip_copilot_metadata(text: str) -> str:
    lines = text.split("\n")
    end = len(lines)
    while end > 0:
        line = lines[end - 1].strip()
        if line == "" or _is_copilot_meta_line(line):
            end -= 1
        else:
            break
    return "\n".join(lines[:end]).strip()


def _parse_copilot(content: str, filename_stem: str) -> Dialogue:
    """
    解析 GitHub Copilot Chat 导出格式：
      # Copilot Chat Conversation Export: <Title>
      **User:** @username
      **Thread URL:** ...
      ## @username
      <用户正文>
      ## Copilot
      TODO N
      <AI 正文>
      ...元数据...
      ## @username
      ...
    """
    # 提取标题
    m = _COPILOT_TITLE_RE.search(content)
    title = m.group(1).strip() if m else filename_stem

    # 按 ## @xxx 或 ## Copilot 切分，保留分隔符以便知道角色
    parts = _COPILOT_TURN_RE.split(content)
    # split 结果：[前置文本, speaker1, body1, speaker2, body2, ...]
    # 即 parts[0] = 前置元数据，之后每两个为 (speaker, body)

    turns: list[Turn] = []
    ai_seq = 0

    i = 1  # 跳过前置文本
    while i + 1 < len(parts):
        speaker = parts[i].strip()   # "@lulufoo" 或 "Copilot"
        body    = parts[i + 1]
        i += 2

        if speaker.startswith("@"):
            body = body.strip()
            if body:
                turns.append(Turn(role="user", todo_n=None, content=body))
        else:  # Copilot
            body = _strip_copilot_metadata(body)
            todo_n, body = _extract_todo_n(body)
            body = _strip_leading_metadata(body)
            if todo_n is None:
                ai_seq += 1
                todo_n = ai_seq
            else:
                ai_seq = todo_n
            turns.append(Turn(role="ai", todo_n=todo_n, content=body))

    return Dialogue(title=title, turns=turns)


# ---------------------------------------------------------------------------
# 格式路由
# ---------------------------------------------------------------------------

# 各格式的特征检测函数与对应解析器
_FORMAT_PARSERS = [
    (
        # GitHub Copilot Chat 导出：标题行含 "Copilot Chat Conversation Export:"
        lambda c: bool(re.search(r"^# Copilot Chat Conversation Export:", c, re.MULTILINE)),
        _parse_copilot,
    ),
    (
        # Cursor 导出：含 **User** 和 **Cursor**/**Copilot** 等加粗说话人
        lambda c: bool(re.search(r"^\*\*(?:User|Cursor|Copilot)\*\*\s*$", c, re.MULTILINE)),
        _parse_cursor,
    ),
    # 可在此追加其他格式：(detector_fn, parser_fn)
]


def parse(content: str, filename_stem: str) -> Dialogue:
    """根据内容特征自动选择解析器。"""
    for detector, parser_fn in _FORMAT_PARSERS:
        if detector(content):
            return parser_fn(content, filename_stem)
    raise NotImplementedError(
        f"未识别的输入格式（文件：{filename_stem}.md）。"
        "请在 _FORMAT_PARSERS 中添加对应的格式检测与解析函数。"
    )


# ---------------------------------------------------------------------------
# 格式化输出（DDM 标准结构）
# ---------------------------------------------------------------------------

# 正文内标题降级：## ~ ###### → 在行首追加 ##（代码围栏内跳过）
_HEADING_RE = re.compile(r"^(#{2,6})\s", re.MULTILINE)
_FENCE_RE   = re.compile(r"^```", re.MULTILINE)


def _downgrade_headings(text: str) -> str:
    """将正文中 ##～###### 降级，代码围栏内不处理。"""
    lines = text.split("\n")
    in_fence = False
    result = []
    for line in lines:
        if line.startswith("```"):
            in_fence = not in_fence
        if not in_fence and re.match(r"^#{2,6}\s", line):
            line = "##" + line
        result.append(line)
    return "\n".join(result)


def render(dialogue: Dialogue) -> str:
    """将 Dialogue 对象渲染为 DDM 标准 Markdown 字符串。"""
    parts: list[str] = []

    parts.append(f"# {dialogue.title}")
    parts.append("")
    parts.append("---")
    parts.append("")

    for turn in dialogue.turns:
        if turn.role == "user":
            header = "## User"
        else:
            n = turn.todo_n if turn.todo_n is not None else "?"
            header = f"## AI（TODO {n}）"

        body = _downgrade_headings(turn.content.strip())

        parts.append(header)
        parts.append("")
        parts.append(body)
        parts.append("")
        parts.append("---")
        parts.append("")

    return "\n".join(parts).rstrip() + "\n"


# ---------------------------------------------------------------------------
# 内嵌文档删减（DDM 阶段一·附，默认开启）
# ---------------------------------------------------------------------------

_EMBEDDED_START_RE = re.compile(r"^````+markdown\s+name=(\S+)", re.MULTILINE)
_EMBEDDED_END_RE   = re.compile(r"^````+\s*$", re.MULTILINE)

KEEP_EMBEDDED = os.environ.get("DDM_KEEP_EMBEDDED_MARKDOWN", "").lower() in ("1", "true")


def strip_embedded_docs(content: str) -> str:
    """删减 ````markdown name=<file> … ```` 块，替换为单行省略说明。"""
    if KEEP_EMBEDDED:
        return content

    result = content
    offset = 0
    removed = 0

    for m_start in _EMBEDDED_START_RE.finditer(content):
        filename = m_start.group(1)
        block_start = m_start.start()
        fence_end   = m_start.end()

        m_end = _EMBEDDED_END_RE.search(content, fence_end + 1)
        if m_end:
            inner = content[fence_end:m_end.start()]
            n_chars = len(inner)
            placeholder = f"（已省略内嵌完整文档 `{filename}`，原约 {n_chars} 字符。）"
            end_pos = m_end.end()
        else:
            inner = content[fence_end:]
            n_chars = len(inner)
            placeholder = (
                f"（已省略内嵌完整文档 `{filename}`，未见结束围栏；"
                f"以下至原文件末尾约 {n_chars} 字符一并省略。）"
            )
            end_pos = len(content)

        adj_start = block_start + offset
        adj_end   = end_pos   + offset
        result    = result[:adj_start] + placeholder + result[adj_end:]
        offset   += len(placeholder) - (end_pos - block_start)
        removed  += 1

    return result


# ---------------------------------------------------------------------------
# 已格式化判定
# ---------------------------------------------------------------------------

_STANDARD_HEADER_RE = re.compile(
    r"^## (?:User|AI（TODO \d+）)$", re.MULTILINE
)
_OLD_STYLE_RE = re.compile(
    r"^\*\*(?:User|AI（TODO \d+）)：\*\*", re.MULTILINE
)


def already_normalized(content: str) -> bool:
    """
    粗判：若全文仅含标准 ## User / ## AI（TODO N）说话人标记，
    且不含旧式 **User：** 标记，视为已归一化。
    """
    has_standard = bool(_STANDARD_HEADER_RE.search(content))
    has_old_style = bool(_OLD_STYLE_RE.search(content))
    return has_standard and not has_old_style


# ---------------------------------------------------------------------------
# CHAT_ID 提取
# ---------------------------------------------------------------------------

# CHAT_ID-<32位十六进制随机数>，出现在 TODO 1 AI 回复中
_CHAT_ID_RE = re.compile(r"^CHAT_ID-([A-Fa-f0-9]{32})\s*$", re.MULTILINE)


def extract_chat_id(content: str) -> Optional[str]:
    """从原始文件内容中提取第一个 CHAT_ID（对应 TODO 1 回复）。"""
    m = _CHAT_ID_RE.search(content)
    return m.group(1) if m else None


# ---------------------------------------------------------------------------
# Index 管理
# ---------------------------------------------------------------------------
#
# index.json 结构：
# {
#   "version": 1,
#   "entries": {
#     "<chat_id>": {
#       "raw":       "<normalized filename>",   # raw/ 目录下的文件名
#       "distilled": "<distilled filename>"     # distilled/ 目录下的文件名，待填
#     }
#   }
# }

_INDEX_VERSION = 1


def load_index() -> dict:
    if INDEX_PATH.exists():
        data = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
        if data.get("version") == _INDEX_VERSION:
            return data
    return {"version": _INDEX_VERSION, "entries": {}}


def save_index(index: dict) -> None:
    INDEX_PATH.write_text(
        json.dumps(index, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def index_set_raw(chat_id: str, raw_filename: str) -> None:
    """将 chat_id → raw 文件名写入 index，保留已有的 distilled 字段。"""
    index = load_index()
    entry = index["entries"].get(chat_id, {})
    entry["raw"] = raw_filename
    entry.setdefault("distilled", None)
    index["entries"][chat_id] = entry
    save_index(index)


def index_set_distilled(chat_id: str, distilled_filename: str) -> None:
    """将 chat_id → distilled 文件名写入 index（供后续蒸馏阶段调用）。"""
    index = load_index()
    if chat_id not in index["entries"]:
        index["entries"][chat_id] = {"raw": None, "distilled": None}
    index["entries"][chat_id]["distilled"] = distilled_filename
    save_index(index)


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def output_path(source: Path) -> Path:
    return RAW_DIR / f"{source.stem}-normalized.md"


def process_file(path: Path) -> None:
    raw_content = path.read_text(encoding="utf-8")

    if already_normalized(raw_content):
        print(f"  ✅ 已是标准格式，跳过：{path.name}")
        return

    out = output_path(path)
    if out.exists():
        print(f"  ⏭  输出已存在，跳过：{out.name}")
        return

    # 在剥离元数据前提取 CHAT_ID
    chat_id = extract_chat_id(raw_content)

    content = strip_embedded_docs(raw_content)

    try:
        dialogue = parse(content, path.stem)
    except NotImplementedError as e:
        print(f"  ⚠️  {e}")
        print(f"     文件：{path.name}")
        return

    normalized = render(dialogue)
    out.write_text(normalized, encoding="utf-8")
    print(f"  📄 已格式化：{out.name}")

    if chat_id:
        index_set_raw(chat_id, out.name)
        print(f"  🗂  index 已更新：{chat_id} → {out.name}")
    else:
        print(f"  ⚠️  未找到 CHAT_ID，跳过 index 更新")


def main() -> None:
    if not RAW_DIR.exists():
        print(f"错误：raw 目录不存在：{RAW_DIR}", file=sys.stderr)
        sys.exit(1)

    candidates = [
        p for p in sorted(RAW_DIR.glob("*.md"))
        if not p.name.endswith("-normalized.md")
    ]

    if not candidates:
        print("raw/ 目录下没有未格式化的文件。")
        return

    print(f"发现 {len(candidates)} 个待处理文件：")
    for p in candidates:
        print(f"  - {p.name}")
    print()

    for path in candidates:
        print(f"处理：{path.name}")
        process_file(path)

    print("\n完成。")


if __name__ == "__main__":
    main()
