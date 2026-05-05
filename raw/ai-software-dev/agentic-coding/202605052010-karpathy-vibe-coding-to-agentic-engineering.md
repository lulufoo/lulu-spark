# Andrej Karpathy: From Vibe Coding to Agentic Engineering

> 创建时间：2026年5月5日 20:10

> 导航：[distilled](../../../distilled/ai-software-dev/agentic-coding/202605052010-karpathy-vibe-coding-to-agentic-engineering.md) · [digest](../../../digest/ai-software-dev/agentic-coding/202605052010-karpathy-vibe-coding-to-agentic-engineering.md) · [trace](../../../trace/ai-software-dev/agentic-coding/202605052010-karpathy-vibe-coding-to-agentic-engineering.md)

> 原文：[Video](https://www.youtube.com/watch?v=96jN2OCOfLs)

## Feeling Behind as a Programmer
Time: 00:47 - 02:26

Host: You said you've never felt more behind as a programmer. That's startling to hear from you of all people. Was that feeling exhilarating or unsettling?

Karpathy: A mixture of both. I had been using agentic tools like Claude Code and adjacent things for a while. Then December was this clear inflection point — I was on a break with more time, and I started to notice that with the latest models the chunks just came out fine. I kept asking for more and it just came out fine. I can't remember the last time I corrected it. I started trusting the system more and more — and then I was vibe coding. It was a very stark transition. A lot of people experienced AI last year as a ChatGPT-adjacent thing. But you really had to look again as of December, because things changed fundamentally, especially on this agentic, coherent workflow that really started to actually work.

## Software 3.0: LLMs as a New Computing Paradigm
Time: 02:28 - 07:36

Host: You've talked about LLMs as a new computer — not just better software, but a whole new computing paradigm. Software 1.0 was explicit rules, 2.0 was learned weights, 3.0 is this. If that's actually true, what does a team build differently the day they believe it?

Karpathy: Software 1.0 — I'm writing code. Software 2.0 — I'm programming by creating datasets and training neural networks. In 3.0, programming turns to prompting. What's in the context window is your lever over the LLM interpreter, which performs computation in the digital information space. A good example: when Claude Code came out, instead of a shell script, the installation is literally a copy-paste of text you give to your agent. You're working in the 3.0 paradigm — you don't have to precisely spell out every detail. The agent has its own intelligence, follows the instructions, looks at your environment, and debugs things in a loop.

A more extreme example: I built MenuGen to take a photo of a restaurant menu and get pictures of what the dishes look like. Then I saw the Software 3.0 version — literally just give the photo to Gemini and say "use Nanobanana to overlay the items onto the menu." It returned an image with the dishes rendered directly into the pixels of the original photo. My entire MenuGen app shouldn't exist. In 3.0, the neural network does most of the work, your prompt and image are the input, and the output is an image — no app layer needed.

Host: Extrapolating further — what's the 2026 equivalent of building websites in the '90s, mobile apps in the 2010s? What will look obvious in hindsight that's still mostly unbuilt today?

Karpathy: You could imagine completely neural computers — a device that takes raw video or audio into a neural net and uses diffusion to render a unique UI for that moment. In the '50s and '60s it wasn't obvious whether computers would look like calculators or neural nets. We went down the calculator path. But I think that's going to flip — the neural net becomes the host process and CPUs become the co-processor. Neural networks doing most of the heavy lifting, using tool calls as a historical appendage for deterministic tasks. Something extremely foreign as the extrapolation — but we'll probably get there piece by piece.

## Verifiability as the Automation Axis
Time: 09:41 - 15:09

Host: AI will automate faster in domains where the output can be verified. What work is about to move much faster than people realize? What professions do people think are safe but are actually highly verifiable?

Karpathy: Traditional computers can easily automate what you can specify in code. This latest round of LLMs can easily automate what you can verify — because frontier labs train these models as giant RL environments with verification rewards. That's why these models are jagged — they peak in verifiable domains like math and code, and are rough around the edges elsewhere.

The favorite example of jaggedness: a state-of-the-art model like Opus 4.7 can simultaneously refactor a 100,000-line codebase or find zero-day vulnerabilities, yet tells you to walk to a car wash 50 meters away. This tells you two things: something's slightly off, or you need to stay in the loop and treat them as tools.

Part of the jaggedness also reflects what labs chose to put in the data distribution. From GPT-3.5 to GPT-4, chess improved a lot — not just from capability progression, but because a huge amount of chess data made it into the pre-training set. So we're somewhat at the mercy of whatever the labs put into the mix. If you're in the RL circuits, you fly. If you're out of the data distribution, you struggle — and you need to look at fine-tuning.

Host: For a founder today who sees a verifiable domain but worries the labs are already at escape velocity in math and coding — what's your advice?

Karpathy: Verifiability still sets you up for your own fine-tuning, even if labs aren't focusing on that domain directly. If you can create RL environments or examples in your domain, you can pull a lever and get something that works well. There are valuable RL environments not yet in the labs' mix — because they're just not high-priority — and that's an opportunity.

Host: What still feels automatable only from a distance?

Karpathy: I do think ultimately almost everything can be made verifiable to some extent. Even for things like writing, you can imagine a council of LLM judges and get something reasonable. It's more about what's easy or hard. Ultimately — everything is automatable.

## Vibe Coding vs. Agentic Engineering
Time: 15:45 - 18:20

Host: Last year you coined vibe coding. Today we're in something more serious — agentic engineering. What's the difference, and what would you actually call what we're in now?

Karpathy: Vibe coding is about raising the floor for everyone — everyone can vibe code anything, and that's amazing. Agentic engineering is about preserving the quality bar of what existed before in professional software. You're not allowed to introduce vulnerabilities. You're still responsible for your software, but can you go faster? The answer is yes — but how do you do it properly?

I call it agentic engineering because it's an engineering discipline. You have these agents — spiky entities, a bit fallible, a little stochastic, but extremely powerful. How do you coordinate them to go faster without sacrificing your quality bar? The ceiling on agentic engineering capability is very high. People used to talk about the 10x engineer — I think that's magnified a lot more than 10x now.

Host: If we watched two people using Claude Code — one mediocre at it, one fully AI-native — how would you describe the difference?

Karpathy: It's getting the most out of the tools available and investing in your setup. Just like engineers used to get the most from vim or VS Code, now it's Claude Code or Codex. I also think most people haven't refactored their hiring process for agentic engineering capability. If you're still giving algorithmic puzzles, that's the old paradigm. Hiring should look like: give someone a big project, have them implement it, make it secure, then unleash agents to try to break it. Watch how they work in that setting.

## Human Skills That Become More Valuable
Time: 19:29 - 23:30

Host: As agents do more, what human skill becomes more valuable, not less?

Karpathy: Right now agents are intern-level entities. You still have to be in charge of aesthetics, judgment, taste, and oversight. A good example of agent weirdness: in MenuGen, users sign up with a Google account but purchase credits via Stripe. My agent tried to match the Stripe email to the Google email to associate funds — but you can use different emails for each account. That's the kind of mistake agents still make — "why would you use email addresses to cross-correlate funds?"

You have to be in charge of the spec, the plan. Work with your agent to design a very detailed spec — maybe have agents write the docs — but you're responsible for the top-level oversight and categories. I also already forgot API details like `keepdim` vs `keep_dims` or `dim` vs `axis` — that's handled by the intern who has great recall. But you still need to understand the fundamentals: that there's an underlying view and storage in PyTorch, and that copying memory around unnecessarily is inefficient. You're in charge of taste, design, and asking for the right things. Agents fill in the blanks.

Host: Could taste and judgment matter less over time, or will the ceiling just keep rising?

Karpathy: I hope it improves. Right now there's probably no aesthetics reward in the RL. When I look at agent-generated code, I sometimes get a little heart attack — it's bloaty, lots of copy-paste, awkward brittle abstractions. It works, but it's gross. A good example: my microGPT project — I kept trying to prompt the model to simplify more, and it just can't. You can feel yourself outside the RL circuits. But there's nothing fundamental preventing improvement — the labs just haven't done it yet.

## Animals vs. Ghosts: Understanding What LLMs Are
Time: 23:31 - 25:16

Host: You wrote a piece about animals vs. ghosts — we're not building animals, we're summoning ghosts. Jagged forms of intelligence shaped by data and reward functions, not by intrinsic motivation, curiosity, or evolutionary drives. Why does that framing matter?

Karpathy: I'm trying to wrap my head around what these things are — because if you have a good model of what they are or aren't, you'll be more competent at using them. These things are not animal intelligences. If you yell at them, it has no impact — they're not going to work better or worse. It's all statistical simulation circuits — the substrate is pre-training statistics, and then RL bolts on top and increases the appendages. It's a mindset about what's likely to work, what's likely not to work, and how to modify the system. I don't have five obvious outcomes. It's more about being appropriately suspicious and figuring it out over time.

## The Agent-Native World
Time: 25:17 - 27:38

Host: You work with agents that have real permissions, local context, and take action on your behalf. What does the world look like when we all start to live in that world?

Karpathy: Everything has to be rewritten — everything is still fundamentally written for humans. My biggest pet peeve: I use frameworks and libraries that still have docs written for humans. I don't want to do anything — what's the thing I should copy-paste to my agent? Every time I'm told to go to a URL or configure DNS through menus, it's just annoying.

The exciting question is: how do we decompose workloads into sensors and actuators over the world? Make it agent-native — describe it to agents first. And ultimately I think we're going toward a world where there's agent representation for people and organizations — my agent will talk to your agent to figure out meeting details. The infrastructure needs to catch up to that.

## What Still Remains Worth Learning Deeply
Time: 27:39 - 29:42

Host: As intelligence gets cheap, what still remains worth learning deeply?

Karpathy: There was a tweet that blew my mind recently: "You can outsource your thinking but you can't outsource your understanding." I'm still part of the system — information still has to make it into my brain. I feel like I'm becoming a bottleneck of just knowing what we're trying to build, why it's worth doing, how to direct my agents. Something has to direct the thinking — and that's still constrained by understanding.

This is also why I'm excited about LLM knowledge bases — a way for me to process information. Every time I see a different projection onto information, I gain insight. I read an article, my wiki gets built up, I ask questions. These are tools to enhance understanding. You can't be a good director without understanding, because the LLMs certainly don't excel at that. You're still uniquely in charge of understanding.

Host: Thank you so much, Andrej. We really appreciate it.
