#!/usr/bin/env python3
"""
DDM Normalize — 将 .cache/ 下未格式化的对话稿转为 DDM 标准，输出到 raw/。

未归一化稿：放在归档根下 ``.cache/*.md``（可 gitignore，仅作本地/暂存入口）。
已归一化：``raw/<同一 stem>-normalized.md``；同目录不混放未处理稿，便于后续按主题分子目录时再接。

判断是否已处理：原稿文件名不以 ``-normalized.md`` 结尾；若全文已是 DDM 标准则脚本内提示跳过。
若 ``raw/`` 下已存在同 stem 的 ``*-normalized.md``，会覆盖重写。

环境变量：
  COGNITIVE_TRACE_ARCHIVE_ROOT  覆盖默认归档根路径（默认 ~/Code/cognitive-trace-archive）
  COGNITIVE_TRACE_CACHE_DIR        未归一化稿目录（默认 归档根/.cache）
  DDM_TEMPLATE_DIR                归一化输出模板所在目录（默认 归档根/templates）
  DDM_TEMPLATE_FILE                单模板文件名（默认 ddm-normalized-output-template.md，a-b-c.md 与文内标题对应）

成稿结构见该文件：第一节为人/AI 的格式区；第二节「Python 查看区」内两 fenced 块为 `render()` 所读（无则试旧版 `@@@` 行界），其余不参与替换。

输入格式识别与解析器顺序见 _FORMAT_PARSERS；新增来源时在此追加 (detector, _parse_xxx)。
"""

import json
import os
import re
import secrets
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
# 未归一化稿入口；输出仍在 RAW_DIR
CACHE_DIR  = Path(os.environ.get("COGNITIVE_TRACE_CACHE_DIR", str(ARCHIVE_ROOT / ".cache")))
RAW_DIR     = ARCHIVE_ROOT / "raw"
DISTILLED_DIR = ARCHIVE_ROOT / "distilled"
DIGEST_DIR    = ARCHIVE_ROOT / "digest"
INDEX_PATH = ARCHIVE_ROOT / "index.json"
TEMPLATES_DIR = Path(
    os.environ.get("DDM_TEMPLATE_DIR", str(ARCHIVE_ROOT / "templates"))
)
# 人读 + 机读合一单文件，内含 @@@ 边界；见该文件头说明
DDM_TEMPLATE_FILE = os.environ.get("DDM_TEMPLATE_FILE", "ddm-normalized-output-template.md")

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
    re.compile(r"^Turn:\d+～\d+$"),                           # Turn:1～16
    re.compile(r"^Model:\s*.+$"),                             # Model: Claude Sonnet 4.5
    re.compile(r"^-{3,}$"),                                   # 轮次间 --- 边界（新解析器遗留）
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
    从文本开头提取轮次编号。支持两种前缀格式：
      - 'Turn1' / 'Turn1\n正文'（Cursor 导出格式）
      - 'TODO1' / 'TODO1\n正文'（旧式格式）
    返回 (编号, 去掉前缀后的正文)。未匹配时返回 (None, 原文)。
    """
    m = re.match(r"^(?:Turn|TODO)(\d+)\s*", text, re.IGNORECASE)
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


_CURSOR_SPEAKER_SPLIT_RE = re.compile(
    r"^(\*\*(?:User|Cursor|Copilot|AI|Assistant|Claude|GPT)\*\*)\s*$",
    re.MULTILINE,
)


def _parse_cursor(content: str, filename_stem: str) -> Dialogue:
    """
    解析 Cursor 导出 Markdown 格式：
      # Title
      _Exported on ..._
      **User**
      <正文（可含任意数量 --- 水平线）>
      **Cursor**
      TODO N <正文>
      ...元数据...
      **User**
      ...

    说话人行（**User** / **Cursor** 等）是唯一可靠的结构锚点；
    --- 仅视为正文内容，不参与切分。
    """
    # 提取标题：第一个 # 标题行
    title = filename_stem
    for line in content.splitlines():
        m = re.match(r"^#\s+(.+)$", line)
        if m:
            title = m.group(1).strip()
            break

    # 按说话人行切分；segments = [前置文本, speaker1, body1, speaker2, body2, ...]
    segments = _CURSOR_SPEAKER_SPLIT_RE.split(content)

    turns: list[Turn] = []
    ai_seq = 0  # 用于无 TODO N 时自动编号

    i = 1  # 跳过前置文本（导出元数据等）
    while i + 1 < len(segments):
        speaker = segments[i].strip()
        body    = segments[i + 1].strip()
        i += 2

        if _CURSOR_USER.match(speaker):
            if body:
                turns.append(Turn(role="user", todo_n=None, content=body))

        elif _CURSOR_AI_NAMES.match(speaker):
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
# 手工 / 转写：加粗说话人 + 全角/半角冒号（可同行起正文；可有 ## TurnN 小节）
# ---------------------------------------------------------------------------

# 行首：**User`：**` / `**AI`：**` 或 **AI…**`：`（含 **AI执行结果（Turn20）**`：` 等）
_BOLD_USER_OR_AI_LINE = re.compile(
    r"(?m)^(\*\*User\*\*|\*\*AI[^*]*\*\*)[：:]\s*",
)


def _parse_bold_colon_speaker(content: str, filename_stem: str) -> Dialogue:
    """
    解析 **User`：**` / `**AI`：**` 及 **AI…**`：` 行首标记的转写稿。
    标记可与首段正文同列；`## TurnN` 仅作为正文中的标题保留。
    """
    matches = list(_BOLD_USER_OR_AI_LINE.finditer(content))
    if not matches:
        return Dialogue(title=filename_stem, turns=[])

    title = filename_stem
    for line in content[: matches[0].start()].splitlines():
        m = re.match(r"^#\s+(.+)$", line)
        if m:
            title = m.group(1).strip()
            break

    turns: list[Turn] = []
    ai_seq = 0
    for i, m in enumerate(matches):
        label = m.group(1)
        is_user = label == "**User**"
        start = m.end()
        end = matches[i + 1].start() if i + 1 < len(matches) else len(content)
        body = content[start:end].strip()
        if is_user:
            if body:
                turns.append(Turn(role="user", todo_n=None, content=body))
        else:
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
# 格式路由：前者未命中时试后者；仅改「认不认得」时只调 lambda 里的正则
# ---------------------------------------------------------------------------

_FORMAT_PARSERS = [
    # GitHub Copilot Chat 导出
    (
        lambda c: bool(re.search(r"^# Copilot Chat Conversation Export:", c, re.MULTILINE)),
        _parse_copilot,
    ),
    # Cursor 等：说话人独占一行、行尾无冒号与同行正文
    (
        lambda c: bool(re.search(r"^\*\*(?:User|Cursor|Copilot)\*\*\s*$", c, re.MULTILINE)),
        _parse_cursor,
    ),
    # 转写稿：**User`：**` / `**AI`：**` 或 **AI…**`：`
    (
        lambda c: bool(
            re.search(r"^(?:\*\*User\*\*|\*\*AI[^*]*\*\*)[：:]", c, re.MULTILINE)
        ),
        _parse_bold_colon_speaker,
    ),
]


def parse(content: str, filename_stem: str) -> Dialogue:
    """按 _FORMAT_PARSERS 顺序匹配解析器。"""
    for detector, parser_fn in _FORMAT_PARSERS:
        if detector(content):
            return parser_fn(content, filename_stem)
    raise NotImplementedError(
        f"未识别的输入格式（文件：{filename_stem}.md）。请在 _FORMAT_PARSERS 中增加 (detector, 解析函数)。"
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


# 每个轮次之间插入的隐藏隔断注释，作为机器解析的唯一可靠边界（与模板中 {{TURN_SEP}} 一致）
_DDM_TURN_SEP = "<!-- DDM:TURN_SEP:v1 -->"

# 缺省模板（TEMPLATES_DIR 下缺文件时回退，与仓库内 MD 内容同步）
_DEFAULT_TEMPLATE_DOCUMENT = """# {{TITLE}}

{{TURN_SEP}}

{{TURN_BLOCKS}}
"""
_DEFAULT_TEMPLATE_TURN = """## {{TURN_HEADER}}

{{TURN_BODY}}

{{TURN_SEP}}


"""

# 旧版单文件亦可能使用 @@@ 行界；优先读下方代码块
_MARK_DOC_BEGIN = "@@@DDM_TEMPLATE_DOCUMENT"
_MARK_DOC_END = "@@@END_DDM_TEMPLATE_DOCUMENT"
_MARK_TURN_BEGIN = "@@@DDM_TEMPLATE_TURN"
_MARK_TURN_END = "@@@END_DDM_TEMPLATE_TURN"
_FENCE_DOC_LANG = "ddm-template-document"
_FENCE_TURN_LANG = "ddm-template-turn"


def _extract_fenced_block(full: str, lang: str) -> Optional[str]:
    """提取 ```<lang> … ``` 的内层文本（不含围栏行）；lang 须与开 fence 第一行一致。"""
    opening = f"```{lang}\n"
    start = full.find(opening)
    if start < 0:
        return None
    a = start + len(opening)
    close = full.find("\n```", a)
    if close < 0:
        return None
    return full[a:close].rstrip("\r")


def _extract_between_markers(full: str, begin: str, end: str) -> Optional[str]:
    """从单模板全文截取 begin、end 两行之间的文本（不含边界行）。兼容旧版 @@@。"""
    i = full.find(begin)
    if i < 0:
        return None
    i += len(begin)
    if i < len(full) and full[i] == "\n":
        i += 1
    j = full.find(end, i)
    if j < 0:
        return None
    return full[i:j].rstrip("\n").rstrip("\r")


def _load_ddm_templates() -> tuple[str, str]:
    """
    从 templates/ddm-normalized-output-template.md 读取：
      优先：ddm-template-document / ddm-template-turn 两个 fenced 代码块内正文；
      否则：@@@…END 旧边界。均失败则回退内嵌默认串。
    """
    path = TEMPLATES_DIR / DDM_TEMPLATE_FILE
    if not path.is_file():
        return _DEFAULT_TEMPLATE_DOCUMENT, _DEFAULT_TEMPLATE_TURN
    full = path.read_text(encoding="utf-8")
    doc_t = _extract_fenced_block(full, _FENCE_DOC_LANG)
    turn_t = _extract_fenced_block(full, _FENCE_TURN_LANG)
    if doc_t is not None and turn_t is not None and doc_t.strip() and turn_t.strip():
        return doc_t, turn_t
    doc_t = _extract_between_markers(full, _MARK_DOC_BEGIN, _MARK_DOC_END)
    turn_t = _extract_between_markers(full, _MARK_TURN_BEGIN, _MARK_TURN_END)
    if doc_t is None or turn_t is None or not doc_t.strip() or not turn_t.strip():
        return _DEFAULT_TEMPLATE_DOCUMENT, _DEFAULT_TEMPLATE_TURN
    return doc_t, turn_t


def render(dialogue: Dialogue) -> str:
    """将 Dialogue 对象按模板渲染为 DDM 标准 Markdown 字符串。"""
    doc_t, turn_t = _load_ddm_templates()
    turn_sep = _DDM_TURN_SEP
    blocks: list[str] = []
    for turn in dialogue.turns:
        if turn.role == "user":
            turn_header = "User"
        else:
            n = turn.todo_n if turn.todo_n is not None else "?"
            turn_header = f"AI（Turn {n}）"
        body = _downgrade_headings(turn.content.strip())
        block = (
            turn_t.replace("{{TURN_HEADER}}", turn_header)
            .replace("{{TURN_BODY}}", body)
            .replace("{{TURN_SEP}}", turn_sep)
        )
        blocks.append(block)
    turn_blocks = "".join(blocks)
    out = (
        doc_t.replace("{{TITLE}}", dialogue.title)
        .replace("{{TURN_SEP}}", turn_sep)
        .replace("{{TURN_BLOCKS}}", turn_blocks)
    )
    return out.rstrip() + "\n"


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
    r"^## (?:User|AI（Turn \d+）)$", re.MULTILINE
)
_OLD_STYLE_RE = re.compile(
    r"^\*\*(?:User|AI（(?:Turn|TODO) \d+）)：\*\*", re.MULTILINE
)
_DDM_SEP_RE = re.compile(r"^<!-- DDM:TURN_SEP:v1 -->$", re.MULTILINE)


def already_normalized(content: str) -> bool:
    """
    判断文件是否已归一化：
    必须同时含有标准说话人标记和 DDM 隐藏隔断注释，且不含旧式标记。
    """
    has_standard = bool(_STANDARD_HEADER_RE.search(content))
    has_sep      = bool(_DDM_SEP_RE.search(content))
    has_old_style = bool(_OLD_STYLE_RE.search(content))
    return has_standard and has_sep and not has_old_style


# ---------------------------------------------------------------------------
# CHAT_ID 提取
# ---------------------------------------------------------------------------

# CHAT_ID-<随机字符串>，出现在 TODO 1 AI 回复中；字符集为 base62（字母+数字）或十六进制
_CHAT_ID_RE = re.compile(r"^CHAT_ID-([A-Za-z0-9]{16,64})\s*$", re.MULTILINE)

# GitHub Copilot Thread URL：**Thread URL:** https://github.com/copilot/c/<uuid>
_COPILOT_THREAD_RE = re.compile(
    r"\*\*Thread URL:\*\*\s+https://github\.com/copilot/c/([0-9a-f-]{32,36})",
    re.IGNORECASE,
)


def extract_chat_id(content: str) -> Optional[str]:
    """
    从原始文件内容中提取唯一对话 ID，按优先级依次尝试：
      1. CHAT_ID-xxx 行（Cursor 导出）
      2. GitHub Copilot Thread URL 末尾 UUID
    """
    m = _CHAT_ID_RE.search(content)
    if m:
        return m.group(1)
    m = _COPILOT_THREAD_RE.search(content)
    if m:
        return m.group(1)
    return None


def generate_chat_id() -> str:
    """当原文无 CHAT_ID 时生成 32 位十六进制 id（`secrets.token_hex(16)`），用于 index 主键。"""
    return secrets.token_hex(16)


def _strip_bucket_prefix(field: str, value: str) -> str:
    p = {
        "raw": "raw/",
        "distilled": "distilled/",
        "digest": "digest/",
    }.get(field, "")
    if p and value.startswith(p):
        return value[len(p) :]
    return value


def entry_bucket_relpath(entry: dict, field: str) -> Optional[str]:
    """
    将 index 中某桶字段解析为**相对该桶根目录**的逻辑路径（POSIX，无开头 /）。
    短名 + 拼接规则（与 schema 一致）：
    1) path_prefix_{field}（如 path_prefix_raw）+ 无「/」的值
    2) 否则 path_prefix + 无「/」的值（各桶同目录时）
    3) 否则值本身为已展开的完整相对路径
    """
    v = entry.get(field)
    if v is None:
        return None
    if not isinstance(v, str):
        return None
    v = _strip_bucket_prefix(field, v)
    per = entry.get(f"path_prefix_{field}")
    if per and field in ("raw", "distilled", "digest") and "/" not in v:
        return f"{per}/{v}"
    prefix = entry.get("path_prefix")
    if prefix and field in ("raw", "distilled", "digest") and "/" not in v:
        return f"{prefix}/{v}"
    return v


def _common_path_from_raw_relpath(raw_relposix: str) -> str:
    """将 raw/ 下相对路径转为 v3 common_path（去掉 basename 的 -normalized）。"""
    raw_relposix = raw_relposix.replace("\\", "/").strip("/")
    if "/" not in raw_relposix:
        b = raw_relposix
        d = ""
    else:
        d, b = raw_relposix.rsplit("/", 1)
    if b.endswith("-normalized.md"):
        b = b[: -len("-normalized.md")] + ".md"
        return f"{d}/{b}" if d else b
    return raw_relposix


def find_chat_id_by_raw_filename(index: dict, raw_relposix: str) -> Optional[str]:
    """在 index 中查找 raw 相对路径（相对 raw/ 根）与 raw_relposix 相等的 chat_id。"""
    ver = index.get("version", 2)
    for cid, entry in index.get("entries", {}).items():
        if ver >= 3:
            cp = entry.get("common_path")
            if not cp:
                continue
            if ver >= 5:
                if "raw" not in entry.get("layers", []):
                    continue
            else:
                if entry.get("raw") is not True:
                    continue
            u = _common_path_from_raw_relpath(raw_relposix)
            if raw_relposix == cp or u == cp:
                return cid
        else:
            r = entry_bucket_relpath(entry, "raw")
            if r == raw_relposix:
                return cid
    return None


# ---------------------------------------------------------------------------
# Index 管理
# ---------------------------------------------------------------------------
#
# index.json 结构：
# - version 2：每字段为桶内相对路径字符串或 null，见 entry_bucket_relpath。
# - version 3：每条目含 "common_path"、raw/distilled/digest 三布尔（是否存在对应文件），
#   见仓库内 index.json 及迁移说明。
#
# load_index 接受 v2 / v3；全新空索引默认 v3。

_SUPPORTED_INDEX_VERSIONS = frozenset({2, 3, 4, 5})

_LAYER_ORDER = ["raw", "distilled", "diagnose", "digest", "trace"]


def load_index() -> dict:
    if INDEX_PATH.exists():
        data = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
        if data.get("version") in _SUPPORTED_INDEX_VERSIONS:
            return data
    return {"version": 5, "entries": {}}


def save_index(index: dict) -> None:
    INDEX_PATH.write_text(
        json.dumps(index, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def index_set_raw(chat_id: str, raw_relposix: str) -> None:
    """将 chat_id → raw/ 下相对路径（POSIX，无 raw/ 前缀）写入 index，保留已有字段。"""
    index = load_index()
    ver = index.get("version", 5)
    if ver >= 3:
        entry = index["entries"].get(chat_id, {})
        cp = _common_path_from_raw_relpath(raw_relposix)
        entry["common_path"] = cp
        if ver >= 5:
            layers = list(entry.get("layers", []))
            if "raw" not in layers:
                layers.insert(0, "raw")
            entry["layers"] = sorted(layers, key=lambda l: _LAYER_ORDER.index(l) if l in _LAYER_ORDER else 99)
            for k in _LAYER_ORDER:
                entry.pop(k, None)
        else:
            entry["raw"] = True
            for k in ("distilled", "digest"):
                if not isinstance(entry.get(k), bool):
                    entry[k] = False
        index["entries"][chat_id] = entry
    else:
        entry = index["entries"].get(chat_id, {})
        entry["raw"] = raw_relposix
        entry.setdefault("distilled", None)
        index["entries"][chat_id] = entry
    save_index(index)


def index_set_distilled(chat_id: str, distilled_filename: str) -> None:
    """将 chat_id → distilled 文件名写入 index（供后续蒸馏阶段调用）。"""
    index = load_index()
    ver = index.get("version", 5)
    if ver >= 3:
        if chat_id not in index["entries"]:
            return
        if ver >= 5:
            layers = list(index["entries"][chat_id].get("layers", []))
            if "distilled" not in layers:
                layers.append("distilled")
            index["entries"][chat_id]["layers"] = sorted(
                layers, key=lambda l: _LAYER_ORDER.index(l) if l in _LAYER_ORDER else 99
            )
            index["entries"][chat_id].pop("distilled", None)
        else:
            index["entries"][chat_id]["distilled"] = True
    else:
        if chat_id not in index["entries"]:
            index["entries"][chat_id] = {"raw": None, "distilled": None}
        index["entries"][chat_id]["distilled"] = distilled_filename
    save_index(index)


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def output_path(source: Path) -> Path:
    return RAW_DIR / f"{source.stem}-normalized.md"


def process_file(path: Path) -> Optional[Path]:
    """
    处理单个文件，返回生成的归一化文件路径；跳过时返回 None。
    """
    raw_content = path.read_text(encoding="utf-8")

    if already_normalized(raw_content):
        print(f"  ✅ 已是标准格式，跳过：{path.name}")
        return None

    out = output_path(path)
    if out.exists():
        out.unlink()
        print(f"  🗑  已删除旧版本：{out.name}")

    # 在剥离元数据前提取 CHAT_ID；无则先查 index 是否已有同 raw 相对路径，再新分配 32 位 hex
    raw_rel = out.relative_to(RAW_DIR).as_posix()
    chat_id = extract_chat_id(raw_content)
    if not chat_id:
        prev = find_chat_id_by_raw_filename(load_index(), raw_rel)
        if prev:
            chat_id = prev
            print(f"  🆔  原文无 CHAT_ID，沿用 index 已有 id：{chat_id}")
        else:
            chat_id = generate_chat_id()
            print(f"  🆔  原文无 CHAT_ID，已生成 32 位 id：{chat_id}")

    content = strip_embedded_docs(raw_content)

    try:
        dialogue = parse(content, path.stem)
    except NotImplementedError as e:
        print(f"  ⚠️  {e}")
        print(f"     文件：{path.name}")
        return None

    normalized = render(dialogue)
    out.write_text(normalized, encoding="utf-8")
    print(f"  📄 已格式化：{out.name}")

    index_set_raw(chat_id, raw_rel)
    print(f"  🗂  index 已更新：{chat_id} → {raw_rel}")

    return out


# ---------------------------------------------------------------------------
# 确认、清理与提交
# ---------------------------------------------------------------------------

def _confirm(prompt: str) -> bool:
    """向用户提问，返回 True 表示确认（y/Y/yes）。"""
    try:
        ans = input(prompt).strip().lower()
    except (EOFError, KeyboardInterrupt):
        print()
        return False
    return ans in ("y", "yes")


def _git(*args: str) -> int:
    """运行 git 子命令，实时输出，返回退出码。"""
    import subprocess
    result = subprocess.run(["git", *args], cwd=ARCHIVE_ROOT)
    return result.returncode


def commit_and_push(processed: list[tuple[Path, Path]]) -> None:
    """
    确认归一化结果后：
      1. 删除 .cache/ 中原始未归一化文件
      2. git add 归一化文件 + index.json
      3. git commit
      4. git push
    processed: [(.cache/ 原始路径, raw/ 归一化路径), ...]
    """
    if not processed:
        return

    print("\n─── 以下文件已归一化 ───")
    for src, out in processed:
        print(f"  暂存：{src} → raw：{out.name}")

    if not _confirm("\n确认无误，删除 .cache 中暂存原稿并提交 raw + index？[y/N] "):
        print("已取消，.cache/ 中暂存原稿保留。")
        return

    for src, _ in processed:
        src.unlink()
        print(f"  🗑  已删暂存原稿：{src.name}（.cache/）")

    # git add
    add_targets = [str(out.relative_to(ARCHIVE_ROOT)) for _, out in processed]
    add_targets.append(str(INDEX_PATH.relative_to(ARCHIVE_ROOT)))
    if _git("add", *add_targets) != 0:
        print("⚠️  git add 失败，请手动处理。", file=sys.stderr)
        return

    # git commit
    names = ", ".join(out.stem for _, out in processed)
    msg = f"normalize: {names}"
    if _git("commit", "-m", msg) != 0:
        print("⚠️  git commit 失败，请手动处理。", file=sys.stderr)
        return

    # git push
    if _git("push") != 0:
        print("⚠️  git push 失败，请手动处理。", file=sys.stderr)
        return

    print("✅ 已提交并推送。")


def main() -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_DIR.mkdir(parents=True, exist_ok=True)

    candidates = [
        p
        for p in sorted(CACHE_DIR.glob("*.md"))
        if not p.name.endswith("-normalized.md")
    ]

    if not candidates:
        print(f".cache/ 下没有未格式化的 .md 文件：{CACHE_DIR}")
        return

    print(f"从 .cache/ 发现 {len(candidates)} 个待处理文件：")
    for p in candidates:
        print(f"  - {p.name}")
    print()

    processed: list[tuple[Path, Path]] = []
    for path in candidates:
        print(f"处理：{path.name}")
        out = process_file(path)
        if out is not None:
            processed.append((path, out))

    print("\n完成。")
    commit_and_push(processed)


if __name__ == "__main__":
    main()
