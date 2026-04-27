# DDM 归一化输出格式

**阅读说明**  
- **格式区**（第一节）：**AI 执行、对照**时只看这里；成稿**不得**出现 `{{…}}`。  
- **程序区**（第二节）：**仅** `normalize.py` 的 `render()` 读取；人写对话稿**不要**复刻本节占位符。  
- **创建时间行**：总标题下写 `> 创建时间：` + **中文日期时间**（`2026年4月25日 15:32` 形态：年月日数字不补零，时分两位、冒号分隔），须与落盘 **basename** 的 12 位 `YYYYMMDDHHMM` 前缀一致；默认东八区，任务另有约定从其约定。  
- **与 CTA 一致（DDM v2.7+）**：可提交 raw 为 `raw/<topic-path>/<ts>-<slug>.md`（**不是** 旧式 `…-normalized.md`）；`index.json` 用 v3 的 `common_path` 指向同一相对路径。导航链接由 Phase 4 注入，Phase 0 可保留占位，见 AADL `ddm-p0-normalize.md`。

---

## 一、AI 查看区：成稿格式（few-shot）

下面为合法外壳；**仿结构**，正文可换；创建时间须与文件名 12 位前缀对应（下为形态示例）。

```markdown
# 总标题

> 创建时间：2026年4月25日 15:32

<!-- DDM:TURN_SEP:v1 -->

## User

用户第一轮，可多行。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

AI 第一轮。

<!-- DDM:TURN_SEP:v1 -->

## User

用户第二轮。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

AI 第二轮。

<!-- DDM:TURN_SEP:v1 -->
```

---

## 二、Python 查看区：`render()` 唯读（fenced 标签勿改）

`normalize._load_ddm_templates()` 只解析本节两个「语言标签为 `ddm-template-document` / `ddm-template-turn`」的代码块，做占位符替换。块内**不要**再嵌 `` ``` ``；改外壳时**勿改**第一行标签名，否则回退内嵌默认串。

```ddm-template-document
# {{TITLE}}

{{TURN_SEP}}

{{TURN_BLOCKS}}
```

```ddm-template-turn
## {{TURN_HEADER}}

{{TURN_BODY}}

{{TURN_SEP}}


```
