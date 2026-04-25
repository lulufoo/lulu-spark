# DDM 归一化输出格式

**阅读说明**  
- **格式区**（第一节）：**AI 执行、对照**时只看这里；成稿**不得**出现 `{{…}}`。  
- **程序区**（第二节）：**仅** `normalize.py` 的 `render()` 读取；人写对话稿**不要**复刻本节占位符。

---

## 一、AI 查看区：成稿格式（few-shot）

下面示例即合法外壳；**仿结构**，正文可换。

```markdown
# 总标题

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
