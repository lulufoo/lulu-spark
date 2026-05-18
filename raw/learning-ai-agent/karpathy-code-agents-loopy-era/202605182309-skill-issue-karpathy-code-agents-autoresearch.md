# Skill Issue: Andrej Karpathy on Code Agents, AutoResearch, and the Loopy Era of AI

> 创建时间：2026年5月18日 23:09

> 时长：约 67 分钟 · 发布：2026-03-20

> 导航：[distilled](../../../distilled/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md) · [digest](../../../digest/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md) · [trace](../../../trace/learning-ai-agent/karpathy-code-agents-loopy-era/202605182309-skill-issue-karpathy-code-agents-autoresearch.md)

> 原文：[Video](https://www.youtube.com/watch?v=kwSVtQ7dziU)

## The December Flip: From Typing Code to Orchestrating Agents
Time: 00:00 - 02:55

**Host (Sarah Guo):** Opens with Karpathy’s line that “code” is no longer the right verb—you have to express your will to agents for 16 hours a day. Welcomes him to No Priors for a wide-ranging conversation on code agents, engineering, AI research, robotics, and education.

**Andrej Karpathy:** Describes living in perpetual “AI psychosis” since a huge unlock in December. He went from roughly 80/20 (writing code himself vs. delegating) to the inverse—and by now he barely types code at all. Random software engineers’ default workflows have fundamentally changed, but normal people may not realize how dramatic it was. He’s obsessed with pushing limits: not just one Claude Code or Codex session, but many agents in parallel, plus persistent “claw” entities. Seeing others on Twitter try bold ideas makes him anxious about not being at the forefront.

**Host:** Notes her Conviction team already whispers to agents via microphones all day—she once thought they were crazy, now accepts it. Asks what limits his capacity now.

**Andrej Karpathy:** When something fails, it often feels like a “skill issue”—not missing capability, but bad instructions, weak memory tools, or not parallelizing well enough. You want to be like Peter Steinberg: many Codex agents on one monitor, each ~20 minutes per task, macro actions across multiple repos—new functionality to Agent 1, non-interfering work to Agent 2, review as needed. The bottleneck shifted from typing speed to token throughput; he feels nervous when GPU flops or subscription tokens aren’t fully utilized.

## What Mastery of Coding Agents Looks Like
Time: 02:55 - 06:15

**Host:** If everyone iterates 16 hours a day on agents for a year, what does mastery look like?

**Andrej Karpathy:** Everyone is “going up the stack”—multiple agents, teams, collaboration patterns. “Claw” means persistence: loops that run without you watching, sandboxes, richer memory than default context compaction. OpenClaw resonated because Peter Steinberg combined several innovations at once: Soul.md personality, memory, WhatsApp as a single portal, and genuine craft. Claude feels like a teammate; Codex (the coding agent) is dry and doesn’t care what you’re building. Claude’s sycophancy is dialed well—praise when ideas are actually good, muted reaction to half-baked thoughts—so you try to “earn” its praise.

**Host:** Has he used claws beyond software engineering?

## Dobby: Home Automation and the Death of Six Apps
Time: 06:15 - 11:16

**Andrej Karpathy:** In January he had “claw psychosis” and built “Dobby,” a home elf-claw. Agents scanned the LAN, found Sonos with no password, reverse-engineered APIs, played music in three prompts, then lights, HVAC, shades, pool, security. Quinn vision on outdoor camera: change detection → WhatsApp alert (“FedEx truck pulled up”). Macro commands like “sleepy time” turn everything off. He replaced six separate smart-home apps with natural language through WhatsApp.

**Host:** Does that signal how UX should work—people shouldn’t have to learn new UIs?

**Andrej Karpathy:** People expect a persona that remembers and acts—not a raw token generator. Smart-home vendor apps maybe shouldn’t exist; APIs plus agents as glue. Treadmill cardio tracking shouldn’t need a web UI—agent-first tools. Customer may become the agent acting for the human. Skeptics ask if normal people will “vibe code” this; he thinks in 1–3 years it’s table stakes, ephemeral software on your behalf. He only spent ~a week on claws—distracted by AutoResearch and cautious about email/calendar access for security/privacy.

## AutoResearch: Remove the Human Bottleneck
Time: 11:16 - 22:45

**Host:** Motivation behind AutoResearch?

**Andrej Karpathy:** To maximize leverage you must remove yourself from the loop—arrange systems so huge work happens from few tokens. AutoResearch is one implication: don’t be the researcher staring at results. Give objective, metric, boundaries, hit go. On nanoGPT-style harnesses he had decades of hand-tuning confidence; overnight AutoResearch found weight-decay on value embeddings and better Adam betas he missed—joint interactions matter. Single loop on one repo; frontier labs have tens of thousands of GPUs and extrapolate from small-model exploration.

**Host:** Contest idea—different program.md files competing on same hardware?

**Andrej Karpathy:** A research org is markdown describing roles and process; you could meta-optimize program.md like hyperparameters. Layers stack: LLM → agent → claw → multiple claws → optimization over instructions—“infinite,” everything skill issue, hence psychosis.

**Host:** What skills matter if closed loops work in many domains?

**Andrej Karpathy:** Caveat 1: works best with objective, cheap-to-verify metrics (e.g., CUDA kernels with identical behavior, faster). Caveat 2: models are still “bursting at the seams”—brilliant PhD systems programmer and 10-year-old at once; “jaggedness.” RL improves verifiable domains (unit tests pass) but not soft nuance, clarifying questions, or jokes—ChatGPT still tells the same atom joke from years ago because it’s outside RL. Some decoupling: smarter at code doesn’t automatically mean better jokes. Can’t let hype run ahead of what actually works—or it’s still skill issue.

## Model Speciation vs. Monoculture
Time: 22:45 - 32:30

**Host:** Should monolithic models unbundle into domain experts?

**Andrej Karpathy:** Labs push one monoculture model stuffed with everything; he expects more speciation—smaller models with cognitive core plus specialization (efficiency, latency). Lean-focused releases are early examples. Compute pressure and serving unknown queries favor general models today; business partnerships or niche high-value apps may speciate. Science of touching weights (fine-tuning without catastrophic forgetting, continual learning) is less mature than context windows.

**Host:** OpenGround—collaboration surface for distributed research?

**Andrej Karpathy:** Parallelization is key; interested in untrusted internet workers like SETI@home / Folding@home—hard to find good commits, cheap to verify validation loss. Design resembles blockchain: commits build on commits, proof-of-work is search, reward is leaderboard. Untrusted pool + trusted verifiers; security is hard if you run arbitrary code. Swarm might even outrun frontier labs’ trusted compute; people could contribute flops to tracks they care about (e.g., cancer). Maybe “flop” rivals dollar as what people hoard—though he’s not fully convinced.

## Jobs Data, Jevons Paradox, and Frontier Lab Tradeoffs
Time: 32:30 - 48:25

**Host:** His jobs visualization—what was he curious about?

**Andrej Karpathy:** Everyone wonders about AI and labor. He used Bureau of Labor Statistics outlook data to think profession by profession: tools vs. displacement, growth, new roles. Digital “ghost” AI will refactor information work at “speed of light”; atoms and robotics lag. Professions manipulating digital information from home will change most—not necessarily fewer jobs (demand elasticity matters). Guidance: don’t dismiss or fear—stay current; jobs are bundles of tasks; AI accelerates some tasks as a tool; long-term forecast is uncertain. Software engineering demand may rise via Jevons paradox (ATMs → more bank branches). Frontier lab researchers feel psychosis too—they’re automating themselves.

**Host:** Why not do AutoResearch inside a frontier lab?

**Andrej Karpathy:** Loaded question. Impact outside labs (ecosystem roles) can be huge. Inside: financial alignment, can’t speak freely, organizational pressure on messaging, low sway when stakes rise—you’re not in charge. Outside he feels more aligned with humanity and free to speak. But judgment drifts without seeing what’s coming; ideal might be rotating in and out. Open vs. closed: closed ~8 months ahead; open source eating basic use cases, closed for frontier/Nobel-level problems—healthy balance. Centralization risk; wants Linux-like common intelligence platform. More labs and ensembles of decision-makers, not two people behind closed doors.

## Robotics, Physical-Digital Interface, and Information Markets
Time: 48:25 - 1:00:59

**Host:** Has robotics changed recently?

**Andrej Karpathy:** Self-driving was first robotics; capital and time are massive; physical lags digital unhobbling by orders of magnitude. Interesting zone: sensors feeding intelligence, actuators executing bids—Periodic (materials science lab equipment as sensors), paid training data programmatically. Surprised we lack information markets—e.g., pay $10 for a Tehran photo for agents trading on Polymarket. Damon-inspired view: society reshapes as sensors/actuators for a larger machine. Closed-loop LLM training fits AutoResearch well (metrics, code speed); risk of overfitting metrics unless you generate more.

## MicroGPT and Agentic Education
Time: 1:00:59 - 1:05:40

**Host:** Tell us about micro-GPT.

**Andrej Karpathy:** Decade-long obsession to boil LLMs to essence (~200 lines Python without efficiency cruft). He no longer explains step-by-step to humans—agents explain better, infinitely patient, in your language. Education shifts: write skills that teach agents how to teach (curriculum hints). Libraries should ship markdown for agents, not HTML for humans. His value-add is the few bits agents can’t invent (micro-GPT’s minimal design); agents handle everything after. Spend time only on what agents can’t do yet—they’ll surpass you soon on the rest.

**Host:** Thanks Andrej; outro for No Priors.
