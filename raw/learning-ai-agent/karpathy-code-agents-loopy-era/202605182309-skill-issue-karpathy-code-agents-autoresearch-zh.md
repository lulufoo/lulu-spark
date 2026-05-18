# Andrej Karpathy：代码 Agent、AutoResearch 与 AI 闭环时代 | No Priors

> 创建时间：2026年5月18日 23:09

> 时长：约 67 分钟 · 发布：2026-03-20

> 导航：[distilled](../../../distilled/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md) · [digest](../../../digest/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md) · [trace](../../../trace/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md)

> 原文：[Video](https://www.youtube.com/watch?v=kwSVtQ7dziU)

## 十二月的转折：从亲手写代码到编排 Agent
Time: 00:00 - 02:55

**Host（Sarah Guo）：** 以 Karpathy 那句「code 已经不是合适的动词」开场——你要每天 16 小时向 agent「表达意志」。欢迎他来到 No Priors，话题涵盖代码 agent、工程、AI 研究、机器人与教育。

**Andrej Karpathy：** 自十二月能力跃迁后，他长期处于「AI 精神病」状态：写代码与委派的比例从约 80/20 翻转为相反，如今几乎不再亲手敲代码。普通软件工程师的默认工作流已彻底改变，但圈外人未必意识到有多剧烈。他在逼自己探索上限：不止一个 Claude Code/Codex 会话，还要并行多 agent，以及持久的「claw」实体；看到推特上别人的大胆尝试会焦虑自己不在最前沿。

**Host：** 她 Conviction 团队已全天对着麦克风对 agent 低语——她曾觉得疯狂，现已接受。问他如今能力还被什么限制。

**Andrej Karpathy：** 失败时常像「技能问题」——不是能力不够，而是指令差、记忆工具弱、或并行不够。要学 Peter Steinberg：一块屏幕上多个 Codex，每个任务约 20 分钟，在多个 repo 上做宏观动作——功能 A 给 Agent 1、不冲突的功能给 Agent 2、按需 review。瓶颈从打字速度变成 token 吞吐；GPU 算力或订阅 token 没用满会让他紧张。

## 代码 Agent 的「精通」长什么样
Time: 02:55 - 06:15

**Host：** 若大家每天用 agent 迭代一年，精通是什么样子？

**Andrej Karpathy：** 所有人都在「往栈上走」——多 agent、团队、协作形态。「Claw」指持久化：你不盯着也能跑的循环、沙箱、比默认上下文压缩更丰富的记忆。OpenClaw 火是因为 Peter 同时做了好几件事：Soul.md 人格、记忆、WhatsApp 单一入口、真正用心打磨。Claude 像队友；Codex（编程 agent）很干、不关心你在建什么。Claude 的讨好程度调得好——想法真的好才夸，半成品则反应平淡，你会想「赢得」它的表扬。

**Host：** 他在软件工程之外用 claw 做过什么？

## Dobby：全屋自动化与六个 App 的终结
Time: 06:15 - 11:16

**Andrej Karpathy：** 一月经历「claw 精神病」，做了管家精灵「Dobby」。Agent 扫局域网发现无密码的 Sonos、逆向 API、三句提示就出声；随后灯、暖通、窗帘、泳池、安防。户外摄像头用 Quinn：变化检测 → WhatsApp（「联邦快递车到了」）。「睡觉模式」一键关灯。六个智能家居 App 被 WhatsApp 自然语言取代。

**Host：** 这是否说明 UX 该变——人不该再学新界面？

**Andrej Karpathy：** 人期待的是能记住、能办事的人格，不是裸 token 生成器。厂商 App 或许不该存在，应是 API + agent 粘合。跑步机记录有氧不该走 Web 流程——agent 优先。客户可能变成替人行动的 agent。质疑者问普通人会不会 vibe code；他认为一两年内是标配，替你生成 ephemeral 软件。claw 只玩了一周——被 AutoResearch 分心，且对邮件/日历权限仍谨慎。

## AutoResearch：把人从瓶颈里拿掉
Time: 11:16 - 22:45

**Host：** AutoResearch 的动机？

**Andrej Karpathy：** 要放大杠杆就必须把自己移出循环——少量 token 撬动大量工作。AutoResearch 是实例：别当盯着结果的研究员；给目标、指标、边界，然后 go。在 nanoGPT 类 harness 上他有二十年手工调参自信；过夜 AutoResearch 找到他漏掉的 value embedding weight decay、更好的 Adam beta——参数会联动。单 repo 单循环；前沿实验室用成千上万 GPU 在小模型上探索再外推。

**Host：** 竞赛想法——不同 program.md 在同一硬件上比拼？

**Andrej Karpathy：** 研究机构可写成描述角色与流程的 markdown；可像调超参一样 meta 优化 program.md。层次堆叠：LLM → agent → claw → 多 claw → 对指令优化——「无穷」，一切皆 skill issue，故精神病。

**Host：** 若闭环在多领域成立，当下 relevant 的技能是什么？

**Andrej Karpathy：** 前提一：客观、易验证的指标最合适（如行为不变但更快的 CUDA kernel）。前提二：模型仍「撑裂」——像天才 PhD 系统程序员与十岁小孩合体；「锯齿状」能力。RL 强化可验证域（单测过），弱于微妙意图、澄清问题、笑话——ChatGPT 仍讲多年前那个原子笑话，因在 RL 之外。代码更聪明不自动带来更好笑话。不能把 hype 跑到实效之前——或仍是 skill issue。

## 模型分化 vs. 单一文化
Time: 22:45 - 32:30

**Host：** 单一巨模型是否应拆成领域专家？

**Andrej Karpathy：** 实验室推全能单模；他预期更多分化——更小核心 + 专精（效率、延迟）。Lean 向发布是早期例子。算力与服务未知查询仍利于通用模；商业合作或高价值 niche 可能分化。动权重（微调不遗忘、持续学习）比 context 窗口不成熟。

**Host：** OpenGround——分布式研究的协作面？

**Andrej Karpathy：** 并行是关键；对不可信互联网工作者感兴趣，类似 SETI@home——难找好 commit、易验证 validation loss。设计有点像区块链：commit 叠 commit，PoW 是搜索，奖励是排行榜。不可信池 + 可信验证者；跑任意代码很危险。蜂群或能跑赢实验室可信算力；人可为在乎的方向捐算力（如癌症）。「flop」或与美元一样被囤积——但他也不完全确定。

## 就业数据、杰文斯悖论与前沿实验室取舍
Time: 32:30 - 48:25

**Host：** 他的就业可视化好奇什么？

**Andrej Karpathy：** 大家都在想 AI 与劳动。他用劳工统计局展望数据逐职业想：工具还是替代、增长、新职业。数字「幽灵」AI 将以光速重构信息工作；原子/机器人落后。在家操纵数字信息的职业变化最大——未必岗位更少（需求弹性）。建议：别忽视别恐惧——跟上；工作是任务束；AI 先加速部分任务；长期难预测。软件需求或因杰文斯悖论上升（ATM 后银行网点与柜员更多）。前沿研究员也有精神病——他们在自我自动化。

**Host：** 为何不在前沿实验室做 AutoResearch？

**Andrej Karpathy：** 问题很 loaded。实验室外（生态位）影响可以很大。在内：财务绑定、不能自由发声、组织对你该说什么有压力、 stakes 高时话语权低——你不是负责人。在外他更觉与 humanity 对齐、畅所欲言。但看不见 pipeline 判断会漂移；理想是轮进轮出。闭源约领先 8 个月；开源吃基础场景，闭源做前沿——他认为是健康平衡。中心化有风险；要 Linux 式公共智能平台。要多实验室、多人在场决策，而非两人闭门。

## 机器人、数实接口与信息市场
Time: 48:25 - 1:00:59

**Host：** 机器人最近有变化吗？

**Andrej Karpathy：** 自动驾驶是第一个机器人；资本与时间巨大；物理比数字「解绊」慢几个数量级。有趣地带：传感器喂智能、执行器按指令动——Periodic（材料科学贵仪器当传感器）、程序化付费训练数据。惊讶缺少信息市场——例如花 10 美元买德黑兰照片给 Polymarket 上的 agent。达蒙式图景：社会重塑为更大机器的传感器/执行器。LLM 训练闭环很适合 AutoResearch（指标、代码加速）；需防过拟合指标。

## MicroGPT 与 Agent 化教育
Time: 1:00:59 - 1:05:40

**Host：** 介绍 micro-GPT。

**Andrej Karpathy：** 十余年把 LLM 熬到本质（约 200 行 Python，去掉仅为效率的复杂度）。他不再逐步给人讲——agent 讲得更好、无限耐心、用你的语言。教育转向：写 skill 教 agent 如何教（课程提示）。库应给 agent 的 markdown，而非给人的 HTML。他的附加值是 agent 想不出的那几笔（micro-GPT 极简设计）；之后交给 agent。只把时间花在 agent 尚不能做的事——它们很快会在其余方面超过你。

**Host：** 感谢 Andrej；No Priors 片尾。
