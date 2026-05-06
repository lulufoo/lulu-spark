# Waymo's Dmitri Dolgov: 20 Million Rides and the Road to Full Autonomy

> 创建时间：2026年5月6日 12:50

> 导航：[distilled](../../../distilled/inbox/waymo-dolgov-20-million-rides/202605061250-waymo-dmitri-dolgov-20-million-rides.md) · [digest](../../../digest/inbox/waymo-dolgov-20-million-rides/202605061250-waymo-dmitri-dolgov-20-million-rides.md) · [trace](../../../trace/inbox/waymo-dolgov-20-million-rides/202605061250-waymo-dmitri-dolgov-20-million-rides.md)

> 原文：[Video](https://www.youtube.com/watch?v=I_0Kuf6Aa2c)

## From the Soviet Union to Stanford: Dmitri's Formation
Time: 00:02 - 02:32

Host (Konstantine Buhler): Almost 20 years building autonomous vehicles. Technically brilliant, incredibly intense, kind and humble. You were born in the Soviet Union, raised in the States, then chose to go back to one of the most prestigious physics programs on the planet in Moscow. How did those early years shape you?

Dmitri Dolgov: My parents went to the same school — that drove the decision. I traveled quite a bit: a year in Japan, high school in the States, then back to college for math and physics in Russia at MIPT. What it really gave me was the technical foundation and, more importantly, the ability to learn independently and explore. That has been the most valuable thing in my career.

## The DARPA Light Switch: Discovering Autonomous Vehicles
Time: 02:33 - 03:50

Host: After your PhD in AI, you were drawn to autonomous vehicles. You were part of the DARPA Grand Challenge in 2005. What drew you in?

Dmitri Dolgov: It was a light switch moment. In college and grad school I had no clear picture of what I wanted to do. Then, right as I was finishing, the urban challenge happened and it just clicked. The technology is fascinating, the mission is incredibly powerful — nothing else comes close — and it's a real product you can experience yourself. It checked all the boxes. Twenty-plus years later, I've never looked back.

## Project Chauffeur: The Crazy Early Days at Google (2009–2011)
Time: 03:51 - 07:04

Host: Waymo started in 2009 as the Google self-driving car project. What were those formative years like?

Dmitri Dolgov: The first couple of years were all about learning the problem space — what it actually means to put an autonomous vehicle on public roads. We set two goals: drive 100,000 miles in full autonomy (unheard-of at the time), and drive 10 routes, each 100 miles long across the Bay Area, each from start to finish without a single intervention. About a dozen of us, 24/7, writing code and building hardware by day, testing at night. It took 18 months to complete both challenges.

Host: You had a reputation for sleeping at the office. How did that period shape your leadership style?

Dmitri Dolgov: Those early days were probably the most fun I've had professionally. You're doing everything — setting up hardware, calibrating sensors, writing the core algorithms, building UIs. You're learning and making progress at an insane rate. After those two years we convinced ourselves this was really worth pursuing, and we doubled down toward a fully autonomous product.

## Surviving the AV Hype Cycle
Time: 07:05 - 09:46

Host: In 2016-17, autonomous vehicles were at the center of a massive hype cycle. Then came a massive slump. Most companies gave up or fell apart. You persisted. For all the builders here — how did you navigate through the hard times?

Dmitri Dolgov: What I've noticed about hype cycles: they follow a breakthrough that drives rapid early progress — convolutional nets, transformers, LLMs — and rapid early investment. In AVs the problem has always been easy to get started but very hard to take all the way to a real product with full autonomy and superhuman safety. Those breakthroughs reshape the beginning of the curve but don't change the long tail.

For us, it came down to understanding that this is not going to be an easy problem — but it is an extremely important one. Today, someone dies in a road crash every 26 seconds worldwide. Knowing the mission is that critical, and not chasing easy wins or silver bullets, gave the team the stamina to go the distance.

## The Waymo Foundation Model: Driver, Simulator, Critic
Time: 09:47 - 12:39

Host: A lot of people are talking about world models. You've had those components for many years. What is Waymo's version of a world model?

Dmitri Dolgov: At the core of our AI ecosystem is what we call the Waymo Foundation Model. It powers three distinct but related pillars: the driver, the simulator, and the critic. The model needs to understand the physical world — physics, dynamics — and what it means to be a good driver: how our agent's actions affect other agents like cars, pedestrians, cyclists. And it needs to be an active participant, not just a passive observer.

Concretely, it's a multimodal world-action-language model. Multimodal because it handles not just images and video but also lidar and radar. A world-action model because it needs precise 3D spatial understanding and must be controllable. Language-aligned so we can pull in the general world knowledge of a VLM — which gives a significant boost in understanding the social semantics of driving.

## End-to-End Is Necessary but Not Sufficient
Time: 12:40 - 15:28

Host: There's a lot of conversation about end-to-end architectures. Is that the right dichotomy for getting to extreme performance and full generalization?

Dmitri Dolgov: To be clear: the Waymo Foundation Model is an end-to-end model — sensors to decisions. The key advantage is that it learns the right rich representations between perception and planning, rather than having that interface engineered, which isn't sufficient for a task like driving.

But I think end-to-end versus "something else" is a false dichotomy. The real question is: end-to-end, and then what else? For a fully autonomous product with superhuman safety at hundreds of millions of miles, a basic vanilla end-to-end system is insufficient. We've augmented learned representations with structured, materialized intermediate representations. That enables runtime validation, richer training and evaluation recipes, closed-loop evaluation and training, and richer reward functions for reinforcement learning — all critical for reaching the last nines of safety.

## Sixth Generation Hardware: Simpler, Cheaper, Scalable
Time: 15:29 - 17:03

Host: There's a sixth generation of the Waymo driver now. Tell us about it — and what was it like the first time you interfaced with it?

Dmitri Dolgov: The sixth generation is our most advanced sensor suite yet. The focus was performance — but equally: simplification, drastic cost reduction, and high-scale volume production. It powers our latest vehicle platform, the Hyundai IONIQ 5 — the "O Hi." We started fully autonomous operations earlier this year; currently employees only, open to all riders later this year.

When I first rode in it, it was one of those new-first moments. The car is designed entirely around the rider experience. Same external footprint as the i Pace, but inside it feels like a living room — huge space in the back, new screens, doors that slide open and auto-open as you approach. I can't wait for it to be in the full fleet.

## Exponential Scaling: 20 Million Rides, 11 Cities
Time: 17:04 - 19:48

Host: You've been through a real phase transition in scaling — 16 years to 100 million miles, six months to 200. What does that exponential growth feel like from the inside?

Dmitri Dolgov: It took 8 years from the day we started fully autonomous operations to the day we were providing rides to the public in 4 cities. Earlier this year we launched 4 cities in a single day. We've now given over 20 million fully autonomous rides — 10 million of those in the last 7 months. That's what exponential scaling looks like.

Launching a new city involves data collection, environment characterization, driver validation — but also starting conversations with local communities, because it's a new product and we have to earn trust. More and more we're seeing the driver generalize incredibly well, so it's really a matter of rigorous evaluation and validation before we deploy.

Host: And how do you personally use Waymo?

Dmitri Dolgov: It's how I get around. I took it here today from Palo Alto to San Francisco on freeways. My three kids love it. On the rare occasions we're in a car driven by a human, they're annoyed — "what was going on there?" There are only two things that get genuine excitement from my kids nowadays: dogs and Waymos.

## Safety as the Non-Negotiable Foundation
Time: 19:49 - 23:11

Host: 1.19 million people die in road accidents every year. You've been intensely focused on safety from the beginning, even while Silicon Valley says move fast and break things. How do you maintain that culture?

Dmitri Dolgov: That number is what drives everyone at Waymo. The status quo is not okay. Building a safety-critical system is fundamentally different from other fields — safety has to be the non-negotiable foundation baked into everything from day one: your model architecture, your training and evaluation recipes, your team's mindset.

It's tempting to focus on capability first and get to 90% quickly. But how you get to the first 90% is a totally different problem from how you get to the next nines. You have to keep that in mind from the start.

Today we drive more than 4 million miles fully autonomously per week. Over 170 million fully autonomous miles, we see that the Waymo driver is more than 13 times safer than a human driver in terms of serious injury-causing collisions. That 13x reduction means we prevent a serious injury every 8 days — and that impact will just grow as we scale.

## Lidar Seeing Around Corners: Emerging Capabilities
Time: 23:12 - 25:04

Host: I heard a story about the lidar detecting footsteps behind a bus. Did that happen?

Dmitri Dolgov: That was one of those moments where I was positively surprised by an emerging capability. In San Francisco, a bus crossed an intersection and stopped partially blocking it. Our light turned green, the Waymo driver started to proceed — and then detected a pedestrian on the other side of the bus. You can't see through a bus: not lidar, not radar, not cameras.

What turned out to be happening: our lidar was bouncing its signal under the bus and getting a sparse return from the movement of the person's feet underneath. That was enough for the Waymo AI to detect that a pedestrian was there, make a prediction about what was going to happen, and keep everyone safe. It blew my mind.

## The Road Ahead: Global Commercialization
Time: 25:05 - 27:09

Audience (Jim): Thinking 5 to 10 years out — what are the milestones? What's going to be different?

Dmitri Dolgov: We're heads down in execution mode. We've transitioned from intentional sequential de-risking of the driver to rapid parallel global commercialization. That means deploying in more cities across the US — 11 today — expanding those existing markets, adding new geographies, and going international. We've announced plans for service in London and Tokyo this year. You'll see us accelerating deployment, all in service of the mission.

Host: From the early days when you could cover a lot of distance with not a lot of technology, through the hard times in autonomous vehicles, to world models, driver-simulator-critic architecture, sixth-gen hardware, safety, and scaling — and most of all, the man who's brought the magic that is Waymo to so many of us. Thank you, Dmitri.
