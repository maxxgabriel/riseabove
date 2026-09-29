# Rise Above — Locked Simulation Design Decisions

> **Status: FINAL CONSOLIDATED LOCKED DESIGN**
>
> This document records the simulation-design decisions explicitly agreed and locked during the audit/discussion.
> It is the design source of truth for intended behaviour. It is **not** a claim that every item is already implemented, and it is **not** the final Codex implementation prompt.
>
> The implementation must be verified against the current repository before work begins. Code and tests describe current implemented reality; this document describes the intended simulation semantics.
>
> Locked areas now include:
>
> 1. Knowledge, perception, staff evaluation, and manager decision-making
> 2. Media truth, credibility, framing, source manipulation, public perception, and persistent media relationships
> 3. Transfer economics, recruitment, negotiation, organisational power, and dynamic risk
> 4. Squad planning, tactical fit, integration, and adaptation
> 5. Contract architecture, negotiation, clauses, promises, and wage structures
> 6. Social opinion, public attention, virality, attractiveness, brand value, history, mythology, and cultural narrative
> 7. Tactical intelligence, match preparation, live adaptation, player state, pressure, and life-to-football effects
> 8. Perspective-safe UI and hidden-state information firewall
> 9. Strongly typed frontend/backend API contracts
> 10. Save-schema evolution, deterministic migrations, and historical continuity
> 11. Imported data, inference, calibration, provenance, and non-circular valuation
> 12. Documentation hierarchy, implementation status, and source-of-truth discipline
> 13. Unified causal simulation architecture: all systems participate in one persistent world
>
> A recurring non-negotiable principle throughout this document is:
>
> ```text
> WORLD TRUTH
> ≠
> ACTOR KNOWLEDGE
> ≠
> PUBLIC KNOWLEDGE
> ≠
> PLAYER-PERSPECTIVE KNOWLEDGE
> ```
>
> Another equally important principle is:
>
> ```text
> Rise Above is one causal simulation.
> It must not behave like a collection of disconnected microfeatures.
> ```

---

# 1. Knowledge, Perception, Staff Evaluation, and Manager Decision-Making

## 1.1 Core rule: people act on beliefs, not hidden world truth

The simulation may know the true state of the world.

Examples of world truth include:

- true current ability
- true potential ability
- true technical, mental, and physical attributes
- true hidden personality attributes
- true injury susceptibility and physical durability
- true relationship values
- true tactical parameters
- true development state
- true internal club decisions
- true motives and intentions

Actors inside the world must **not** be allowed to read those values directly unless that information is legitimately available to them.

The fundamental flow is:

```text
WORLD TRUTH
    ↓
evidence / observation / scouting / training / matches / conversation
    ↓
individual assessments
    ↓
staff and departmental opinions
    ↓
manager interpretation
    ↓
confidence / uncertainty
    ↓
decision
```

The world engine knows truth. Managers, scouts, coaches, analysts, agents, players, journalists, supporters, and other people operate from their own knowledge and beliefs.

This rule is foundational because mistakes, disagreement, discovery, missed talent, bad transfers, late bloomers, and managerial judgement only become meaningful if people can be wrong.

## 1.2 Do not replace omniscience with artificial stupidity

The solution is not to force every actor to remain permanently inaccurate. Knowledge should improve through legitimate evidence.

A manager who has worked with a senior player every day for three years should know that player's current level, habits, tactical suitability, and personality extremely well. A scout who has watched a player twice should know far less.

Conceptually:

```text
Own senior player, long tenure:
- current ability certainty: very high
- tactical fit certainty: very high
- personality certainty: high
- physical behaviour certainty: high
- future potential certainty: still imperfect

Academy player:
- current ability certainty: moderate to high depending on exposure
- tactical fit certainty: moderate
- personality certainty: improving over time
- potential certainty: meaningfully uncertain

Barely scouted external player:
- current ability certainty: low to moderate
- tactical fit certainty: low
- personality certainty: very low
- potential certainty: low
```

A sufficiently skilled and experienced decision-maker can become highly accurate. They must never become magically omniscient merely because the engine has exact values.

## 1.3 Manager licences are competence foundations, not magic knowledge levels

Managerial and coaching licences should matter.

Examples:

- no formal licence / entry qualification
- lower coaching badges
- B licence
- A licence
- Pro licence

However:

```text
Licence ≠ direct access to truth
Licence ≠ guaranteed elite judgement
Licence ≠ exact CA/PA knowledge
```

A licence represents education, exposure to formal coaching methodology, and a competence foundation. Actual judgement should sit on top of that foundation.

A Pro Licence manager can still be poor at judging potential. A lower-licensed manager can still have an exceptional eye for talent.

Example:

```text
Manager A
Licence: Pro
Current ability judgement: excellent
Potential judgement: average
Tactical knowledge: elite
Youth development knowledge: weak
Man management: strong

Manager B
Licence: B
Current ability judgement: good
Potential judgement: exceptional
Tactical knowledge: average
Youth development knowledge: excellent
```

The badge matters, but it does not collapse every manager into the same competence tier.

## 1.4 Player evaluation must be multi-domain

Do not use one universal "judging" number for everything.

The game should distinguish, directly or through derived traits, between different kinds of football judgement.

Relevant domains include:

- current ability judgement
- potential projection
- tactical suitability
- role suitability
- technical assessment
- physical readiness
- mental readiness
- personality/professionalism assessment
- injury/durability understanding
- youth projection
- adaptability assessment
- development trajectory judgement

Not every one of these needs to be a visible user-facing attribute. Some may be derived from staff attributes, licences, role, experience, speciality, age, personality, and history.

The important rule is that a manager can be excellent in one area and weak in another.

Examples:

- elite tactician, mediocre youth evaluator
- excellent academy coach, weak senior-level talent judge
- brilliant current-ability judge, poor long-term projector
- excellent technical evaluator, biased against physically weak players
- excellent analyst, poor personality reader

## 1.5 Projection should be a belief dossier, not hidden PA with noise

The manager should not internally store:

```text
Player X PA = 181
```

and then merely display a fuzzy range.

The system should model a football judgement.

Conceptually:

```text
PLAYER PROJECTION

Current level:
- strong Championship player
Confidence:
- very high

Short-term projection:
- capable of contributing in the Premier League
Confidence:
- high

Long-term ceiling:
- likely Premier League starter
- possible Champions League-level player
Confidence:
- medium

Development direction:
- positive

Risks:
- physical development uncertain
- decision-making inconsistent

Best projected roles:
- inside forward
- winger

Evidence:
- training: strong
- senior matches: limited
- youth matches: extensive
- coach reports: positive
- analytics: promising
```

The underlying engine may represent this with distributions, ranges, probabilities, and confidence values. The important point is that the belief represents what the evaluator thinks, why they think it, and how certain they are.

## 1.6 Different staff members can legitimately disagree

A club must not behave like a single shared brain.

The same player can be evaluated differently by:

- manager
- assistant manager
- head of youth development
- first-team coach
- scout
- recruitment analyst
- performance analyst
- sporting director
- medical staff

Example:

```text
True long-term ceiling: elite

Manager:
- projection: good top-flight player
- confidence: high

Assistant:
- projection: strong top-flight player
- confidence: medium

Academy director:
- projection: possible elite player
- confidence: high

Scout:
- projection: high-level top-flight player
- confidence: medium

Analyst:
- progression indicators: elite
- direct potential assessment: limited
```

This disagreement is desirable. It creates real club mistakes, internal debate, missed talent, and staff reputation.

## 1.7 The manager interprets staff information rather than automatically averaging it

A club's conclusion should not simply be:

```text
(manager opinion + assistant opinion + scout opinion) / 3
```

The manager should decide how much weight to give each source.

Factors include:

- trust in that staff member
- staff competence
- role relevance
- years worked together
- previous accuracy
- professional reputation
- current evidence
- manager ego
- manager delegation tendency
- manager philosophy
- organisational hierarchy
- whether the staff member was inherited or personally hired

Example: a manager has worked with an assistant for twelve years and trusts them deeply, but has just inherited an academy director from the previous regime. The academy director may be objectively better at potential evaluation, yet the manager can still discount that opinion. If the academy director later proves correct after the player succeeds elsewhere, that history should matter.

## 1.8 Departments matter

Knowledge quality is partly organisational.

A wealthy elite club may have:

- strong coaching staff
- high-quality analysts
- extensive scouting network
- strong sports science
- excellent medical department
- deep historical data
- strong youth staff

A small club may have:

- one competent manager
- weak assistant staff
- one good scout
- little analytics
- limited sports science
- narrow geographical coverage

The elite club should generally have broader and more precise organisational knowledge. That does not mean it should always be correct. A smaller club can have one exceptional evaluator and discover someone the large clubs missed.

## 1.9 Exposure changes certainty

Assessment quality should improve with legitimate exposure.

Sources of exposure include:

- daily training
- reserve/youth training
- first-team appearances
- youth matches
- loans
- scouting assignments
- video review
- analytics
- international matches
- staff reports
- medical history
- conversations
- dressing-room observation
- repeated opposition encounters

The amount and type of exposure matters. A manager may know a player's tactical habits very well but have little confidence about long-term physical development. A scout may have a strong current ability read but almost no personality knowledge.

## 1.10 Potential should remain uncertain

Potential is future possibility, not a database fact available to people.

Even elite evaluators should retain uncertainty, especially with young players.

The engine can have a true developmental ceiling or latent development parameters. People should hold projections.

Those projections can be:

- correct
- too optimistic
- too pessimistic
- biased
- outdated
- revised after evidence
- split between staff members

This uncertainty is central to:

- wonderkids being missed
- late bloomers
- academy mistakes
- bad releases
- unexpected stars
- overpriced prospects
- scouting disagreements
- managerial regret
- retrospective media narratives

## 1.11 Philosophy changes how evidence is interpreted

Accuracy is only part of decision-making. A manager's football philosophy determines what they value.

The same player can produce the same evidence and still generate different conclusions.

Example player:

- 17 years old
- technically excellent
- physically weak
- excellent training attitude
- inconsistent senior minutes

Possible managerial interpretations:

```text
Technique-first manager:
"Build around him."

Physical manager:
"Not ready."

Youth developer:
"Give him six months."

Manager under relegation pressure:
"I cannot afford the development time."
```

This is not necessarily evaluator incompetence. It is value judgement and context.

Decision-making therefore depends on both:

```text
How accurately do you understand the player?
+
What do you value?
```

## 1.12 Context affects decisions even when perception is accurate

A manager can correctly recognise a player's ability and still make a different decision because of:

- tactical system
- current results pressure
- next opponent
- fixture congestion
- dressing-room hierarchy
- promised minutes
- leadership
- squad registration
- homegrown rules
- board expectations
- finances
- age profile
- replacement availability
- title race
- relegation fight
- youth policy
- long-term squad plan

Therefore:

```text
Correct evaluation ≠ automatic selection
Correct evaluation ≠ automatic promotion
Correct evaluation ≠ automatic transfer
```

## 1.13 Players and agents also act on beliefs

The same anti-omniscience rule applies to players evaluating career decisions.

A player should not decide whether to join a club by comparing hidden true CA values against every player in the destination squad.

Instead, a player and agent should reason from things such as:

- player's self-perception
- agent's assessment
- destination club reputation
- manager reputation
- visible squad competition
- public reputation of competing players
- known recent performances
- promised squad status
- conversations with manager/director
- expected tactical role
- club ambition
- wages
- household/relocation considerations
- language/culture
- existing relationships
- other offers
- what the player has heard

The player can therefore misjudge an opportunity.

Examples:

- believes they will start, then discovers stronger competition
- fears being buried and rejects a move that would actually have suited them
- overestimates own level
- accepts a role promise that later becomes unrealistic
- trusts an agent whose assessment is wrong

Those are desirable career stories.

## 1.14 Knowledge should be revisable

Beliefs should change when new evidence arrives.

Examples:

- exceptional training
- poor match performances
- successful loan
- serious injury
- tactical role change
- physical maturation
- repeated inconsistency
- new staff arrival
- manager change
- international breakthrough
- reliable scouting report
- poor scouting report later disproved

The system should preserve enough history to explain why an assessment moved.

Where useful:

```text
Previous projection:
"rotation-level top-flight player"

New projection:
"possible regular starter"

Why:
- strong loan season
- improved physical development
- positive training reports
- successful senior appearances
```

## 1.15 Design consequence: remove hidden-truth shortcuts from decisions

Existing or future systems must be audited for direct access to truth where perception should be used.

Particular danger areas include:

- team selection
- youth call-ups
- reserve emergency selection
- squad planning
- succession planning
- transfer targeting
- transfer valuation
- player move decisions
- player expected-minutes calculations
- staff recommendations
- tactical role suitability
- academy promotion/release
- contract status decisions

A system may use truth to simulate reality. It must not use truth to represent what an actor knows.

## 1.16 Desired emergent stories

This architecture should naturally allow stories such as:

```text
academy player underestimated
→ released
→ smaller club signs them
→ strong performances
→ larger scouts revise opinion
→ former club criticised later
```

or:

```text
academy director strongly recommends player
→ manager distrusts inherited staff member
→ player sold
→ player becomes elite
→ director's reputation rises
→ old decision resurfaces in media
```

or:

```text
manager rates current level correctly
→ does not value player's profile
→ another manager signs them
→ player flourishes in a different system
```

The story comes from judgement, uncertainty, relationships, and football context, not scripted narrative events.

---

# 2. Media Truth, Credibility, Framing, Source Manipulation, Public Perception, and Persistent Relationships

## 2.1 Core rule: "was the journalist right?" is not one question

Media accuracy must not collapse into:

```text
Did the eventual transfer happen?
yes = journalist correct
no = journalist wrong
```

The system must distinguish several layers.

At minimum:

- what was objectively true at publication time
- what the source actually said
- what the journalist genuinely knew
- how strong the journalist's evidence was
- how the journalist framed the information
- whether the journalist intentionally distorted it
- what the audience inferred
- what eventually happened
- how the public later remembers the story

These are separate.

## 2.2 Explicit truth categories

Stories and claims should support distinctions such as:

### False

The material claim was factually wrong.

### Misleading

The wording may be technically true, but the presentation creates a materially false or distorted impression.

### Manipulated

A source deliberately planted, shaped, exaggerated, omitted, or weaponised information and the journalist published it. The journalist may or may not realise they are being used.

### Accurate at the time

The story correctly described reality when published, but the world changed afterward.

Example:

```text
Monday:
club expects deal to close
agent expects agreement
medical tentatively booked

Journalist:
"Club expects Maxx to sign this week."

Wednesday:
another club enters

Thursday:
player changes mind

Friday:
deal collapses
```

The original report was not necessarily false.

## 2.3 Separate claim accuracy from outcome accuracy

A claim must be evaluated against the exact thing claimed.

Example:

```text
Claim:
"Arsenal contacted the player's agent."

Truth:
Yes.

Later outcome:
Player signs elsewhere.
```

The original journalist was accurate. The failed transfer must not retroactively turn the original report into a miss.

Likewise:

```text
Claim:
"Arsenal have submitted a formal bid."

Truth:
No formal bid existed.

Later:
Arsenal eventually submit one and sign the player.
```

The original report was still wrong at the time. Luck does not convert bad reporting into good reporting.

## 2.4 Separate truth, framing, intent, source manipulation, perception, and outcome

The following dimensions should be independently representable:

```text
TRUTH
FRAMING
INTENT
SOURCE MANIPULATION
PUBLIC PERCEPTION
EVENTUAL OUTCOME
```

Possible combinations include:

- honest journalist, accurate story, later looks wrong
- honest journalist, manipulated by source
- hostile journalist, factually accurate story, unfair framing
- dishonest journalist, false story
- sensational journalist, misleading headline built on true details
- dishonest journalist, accidentally correct eventual prediction
- excellent reporter, poor public reputation
- popular reporter, weak professional credibility
- obscure reporter, excellent source access

## 2.5 Technically true can still be misleading

Example:

Club says:

> "No formal bid has been received."

This may be literally true.

Internally:

- clubs have already held informal talks
- valuation has been discussed
- agent knows a formal offer is expected

A headline such as:

> "Club dismisses transfer speculation"

could therefore be misleading even though the quoted statement is factually accurate.

Another example:

> "Maxx has not asked to leave."

True. But perhaps the player's agent privately told the sporting director the player wants a move.

The system should be capable of recognising the distinction between literal truth and the impression created.

## 2.6 Sources can strategically manipulate media

Sources should have motives.

Examples:

- agent manufactures rival interest to improve wages
- club tests the market by leaking availability
- director leaks to pressure manager
- manager leaks to protect themselves
- player camp leaks dissatisfaction
- owner leaks takeover confidence
- source selectively reveals one part of an internal dispute
- source gives technically true but incomplete information
- source exaggerates urgency
- source passes outdated information
- source deliberately plants false information

An honest journalist can therefore publish a bad story because they trusted a source.

That should affect:

- source reliability
- journalist-source relationship
- journalist caution in future
- public perception
- professional reputation
- editorial behaviour

The journalist's own honesty and the source's honesty are distinct.

## 2.7 Journalist credibility must be contextual

Do not use a single universal credibility score as the decisive measure.

A journalist may be:

```text
General reputation: medium

Arsenal transfer reporting:
excellent

Arsenal tactical analysis:
average

Chelsea transfer reporting:
poor

Agent-network access:
excellent

Manager access:
poor

Sporting-director access:
excellent
```

Relevant dimensions can include:

- overall professional standing
- club-specific source access
- topic expertise
- transfer accuracy
- tactical knowledge
- youth knowledge
- local knowledge
- source-network strength
- historical claim accuracy
- employer reputation
- language/geographical knowledge
- reliability of current source
- experience on the beat
- public audience reputation
- professional reputation among peers

The same journalist can be trusted on one subject and distrusted on another.

## 2.8 Reach, fame, credibility, expertise, and entertainment value are different

These should not collapse into one number.

A media figure can have:

```text
Reach: enormous
Credibility: mediocre
Entertainment value: high
Transfer expertise: low
Club-politics expertise: high
```

People may consume and amplify them despite not trusting them.

Likewise an obscure local reporter can have low reach, exceptional local access, excellent source network, and strong professional credibility.

## 2.9 Audience belief is contextual

A supporter should not primarily reason:

```text
Outlet credibility = 82
therefore story believable
```

Belief should consider:

- journalist identity
- outlet identity
- topic
- club
- journalist history on that topic
- journalist history with that club
- source reputation if inferable
- claim type
- evidence/corroboration
- the reader's prior belief
- the reader's club affiliation
- rivalry
- knowledge
- credulity
- personality
- emotional desirability of the claim
- personal history with the journalist
- current narrative climate

Two supporters can rationally reach different conclusions from the same story.

## 2.10 Believing, sharing, and reacting are separate decisions

This is mandatory.

A person can:

- believe and share
- believe and stay silent
- doubt but share
- doubt and mock
- disbelieve but amplify because it hurts a rival
- share merely because it is entertaining
- quote-post to challenge it
- save it for later
- ignore it

Example:

```text
belief probability: low
share probability: high

Reason:
"This probably isn't true, but it would be hilarious if it were."
```

This is essential for realistic social behaviour.

## 2.11 Public perception can be wrong about the journalist

A journalist may report several accurate situations that later collapse.

Supporters may conclude:

> "This reporter always gets things wrong."

Internally, the claims may have been correct at publication time.

Therefore maintain separation between:

- actual professional accuracy
- source quality
- public credibility
- club-specific audience opinion
- professional peer reputation

A good reporter can have a bad public image. A bad reporter can have a strong public following.

## 2.12 Organisations are not single minds

Do not store:

```text
Club opinion of journalist = 62
```

as the authoritative relationship.

A club contains people.

Possible simultaneous relationships:

```text
Manager → Journalist:
hostile

Sporting Director → Journalist:
trusting

Media Officer → Journalist:
professional and useful

Owner → Journalist:
neutral
```

The same rule applies to the journalist's view of those people.

This matters for:

- access
- leaks
- interviews
- exclusives
- briefings
- retaliation
- denials
- source protection
- press-conference behaviour

## 2.13 Media relationships are persistent, directional, and causal

A relationship is directional.

```text
A → B
```

is not the same as:

```text
B → A
```

Examples:

- journalist hates manager; manager barely remembers journalist
- player believes reporter has an agenda; reporter believes relationship is normal
- journalist respects rival journalist; rival resents them
- manager distrusts outlet; outlet is neutral toward manager

Do not automatically mirror relationship values.

## 2.14 Relationship dimensions should be richer than "likes/dislikes"

Useful media-world relationship dimensions can include:

- trust
- respect
- access
- hostility
- professional regard
- familiarity
- grievance
- source confidence
- confidentiality confidence
- competitiveness
- dependence
- favourability
- willingness to cooperate

Not every relationship needs every dimension stored explicitly. Some can be derived.

The important requirement is that future behaviour comes from meaningful history rather than one generic affinity number.

## 2.15 Relationships remember why they changed

A relationship should preserve causal memories.

Possible causes:

- public insult
- misleading article
- inaccurate reporting
- accurate but damaging reporting
- confidential information leaked
- source identity exposed
- repeated hostile questioning
- interview access denied
- public praise
- exclusive granted
- source protected successfully
- quote taken out of context
- correction published
- reporter refused to correct
- player criticised publicly
- manager humiliated reporter
- journalist contradicted another journalist
- scoop stolen
- agent manipulated journalist
- long-term professional cooperation

The reason matters because different grievances create different future behaviour.

## 2.16 Media-player, media-manager, and media-club relationships affect behaviour

A hostile journalist may:

- choose harsher framing
- ask more confrontational questions
- investigate negative angles more aggressively
- offer less benefit of doubt
- highlight contradictions
- amplify criticism
- chase follow-ups harder

This must not mean they automatically invent facts.

Bias can affect:

- what they investigate
- what they publish
- what angle they choose
- what questions they ask
- how prominently they frame something

Truth constraints still apply.

A hostile reporter can still publish praise when evidence is overwhelming. A friendly reporter can still publish damaging facts.

## 2.17 Players and managers can develop their own media grudges

Example:

```text
player neutral toward journalist
→ repeated criticism
→ misleading headline
→ hostile press-conference exchange
→ player distrusts journalist
```

Possible player responses later:

- calm answer
- guarded answer
- challenge premise
- sarcasm
- refuse to answer
- public criticism
- bypass journalist
- give access to competing reporter

These choices then create further media events. The relationship is part of the simulation, not cosmetic flavour.

## 2.18 Media-vs-media relationships and rivalries exist

Journalists and outlets can have professional rivalries.

Examples:

- competing transfer reporters
- local paper versus national outlet
- data outlet versus tabloid
- fan channel versus mainstream broadcaster
- journalists competing for the same club sources
- reporters contradicting each other's claims

Possible behaviour:

- race for exclusives
- seek independent confirmation
- publicly contradict a claim
- reference competitor reporting
- mock past misses
- defend a respected peer
- accuse competitor of club bias
- cultivate alternative sources
- become reluctant professional allies
- collaborate when interests align

Again, not a hardcoded "beef mode." The rivalry should emerge from history, competition, personality, reputation, and shared sources.

## 2.19 Media conflicts can be long-lived and can cool or reignite

A feud can:

- intensify
- fade
- become professional respect
- disappear after job changes
- reignite years later
- survive a club move
- transfer to a new outlet
- become one-sided
- become institutional if multiple people reinforce it

Example:

```text
Reporter clashes with manager at Club A
→ manager freezes reporter out
→ reporter changes outlet
→ manager later moves to Club B
→ they meet again years later
→ old history affects first interaction
```

Persistent world memory should make this possible.

## 2.20 Journalists can be professionally respected but publicly disliked

Reputation should be audience-specific.

A reporter can simultaneously be:

- respected by other journalists
- trusted by agents
- disliked by one manager
- useful to a sporting director
- mocked by rival supporters
- trusted by local supporters
- seen as sensational by neutral audiences

There is no contradiction. That complexity is desired.

## 2.21 Source relationships are relationships too

Journalist-source ties should have their own history.

Relevant properties include:

- strength
- reliability
- confidentiality
- recent usage
- topic relevance
- source motive
- history of correct information
- history of manipulation
- whether the journalist protected the source
- whether the journalist exposed or embarrassed the source
- whether the source has reason to use the journalist strategically

A source can be:

- highly connected but manipulative
- honest but poorly informed
- generally reliable but wrong this time
- accurate on one topic and weak on another

## 2.22 Public source inference is not source omniscience

The journalist may know the actual source identity. The audience usually should not.

Supporters and other media may infer:

- "this sounds like the agent"
- "this reporter is close to the sporting director"
- "this likely came from the player's camp"

That inference can influence belief. It must remain an inference unless the source becomes public.

## 2.23 Corrections, denials, and follow-ups should preserve semantic history

A story should remain part of a continuing information history.

Possible chain:

```text
rumour
→ corroboration
→ denial
→ journalist follow-up
→ formal bid
→ talks
→ collapse
→ retrospective
```

Each stage should preserve:

- what was claimed
- by whom
- based on what
- how confident the claim was
- how audiences reacted
- what later evidence changed
- who gained or lost credibility

## 2.24 Desired emergent media stories

### Honest reporter looks wrong

```text
reporter accurately reports advanced talks
→ deal unexpectedly collapses
→ fans mock reporter
→ professional peers know report was legitimate
→ public credibility falls while professional respect remains high
```

### Reporter is used by an agent

```text
agent invents rival interest
→ trusted reporter publishes
→ club increases renewal offer
→ story later exposed
→ reporter-source relationship damaged
→ journalist becomes more cautious with that agent
```

### Genuine media feud

```text
manager publicly mocks reporter
→ reporter becomes hostile
→ club media officer still needs reporter
→ sporting director continues leaking
→ manager blocks access
→ outlet escalates questioning
→ relationship persists across seasons
```

### Reporter rivalry

```text
Reporter A breaks story
→ Reporter B publicly says it is false
→ A later proves correct
→ B loses credibility with some audiences
→ A remembers contradiction
→ future scoops become competitive
```

### Hostile framing without fabricated facts

```text
player performs poorly
→ hostile reporter writes critical but factually grounded piece
→ friendly reporter writes sympathetic analysis
→ both stories are supported by the same match
→ audiences divide based on relationship and prior belief
```

---

# 3. Cross-System Principles Shared by Both Locked Areas

## 3.1 Reality, knowledge, interpretation, and action are separate layers

For both football evaluation and media:

```text
REALITY
↓
AVAILABLE INFORMATION
↓
PERSONAL INTERPRETATION
↓
BELIEF
↓
ACTION
↓
CONSEQUENCE
↓
NEW REALITY
```

Do not collapse these layers.

## 3.2 People can be wrong for understandable reasons

Wrong decisions should generally have causes.

Examples:

- weak evidence
- poor evaluator
- good evaluator with bad information
- source manipulation
- philosophical bias
- lack of exposure
- outdated information
- trusted colleague gave bad advice
- emotional loyalty
- rivalry
- pressure
- deliberate deception

Avoid arbitrary stupidity inserted only to create drama.

## 3.3 People can disagree without one being irrational

Two competent people can interpret the same evidence differently.

This applies to:

- manager versus academy director
- scout versus analyst
- journalist versus journalist
- supporter versus supporter
- manager versus sporting director
- player versus agent

The game should support genuine disagreement.

## 3.4 Relationships are directional and historical

Whenever a relationship materially affects behaviour:

- A's view of B can differ from B's view of A
- the current state should have causal history
- future behaviour should be influenced by that history
- old history may fade but should not vanish arbitrarily

## 3.5 Global scores are allowed only as summaries, not universal truth

A global reputation, fame, reach, or standing value may exist for convenience. It must not replace contextual evaluation.

Examples:

```text
global journalist reputation
≠ transfer credibility with Arsenal

global player reputation
≠ manager's tactical trust

club reputation
≠ player belief about expected minutes

outlet credibility
≠ supporter belief in a particular journalist's claim
```

## 3.6 The purpose is emergent history, not scripted drama

These systems exist so that stories emerge naturally.

The game should not force:

- "wonderkid rejected" events
- "journalist feud" events
- "manager regret" events
- "media controversy" events

Instead:

```text
state
→ evidence
→ belief
→ decision
→ relationship change
→ consequence
→ later reinterpretation
```

The resulting history becomes the story.

---

# 4. Audit Targets Implied by These Decisions

When implementation work begins, the codebase should be audited for violations of these locked rules.

## Football knowledge / decision-making

Priority targets include:

- direct `ca` use in manager decisions
- direct `pa` use in club planning
- youth call-up ordering
- emergency squad selection
- foreign-player trimming
- player expected-minutes calculations
- transfer move utility
- academy release/promotion
- succession planning
- scouting-to-recruitment handoff

## Media

Priority targets include:

- belief functions that rely mainly on outlet credibility
- journalist reputation flattened into one number
- claim evaluation based only on eventual transfer outcome
- source reliability without topic/context
- organisation-wide media relationships
- symmetric relationship assumptions
- stories that lose claim-time truth
- misleading framing not represented
- public perception treated as equal to professional accuracy

---

# 5. Non-Negotiable Summary

## Football intelligence

```text
People do not read hidden truth.

Managers know players through:
licence
+ skill
+ experience
+ exposure
+ staff reports
+ department quality
+ trust
+ evidence
+ philosophy
+ context.

Potential remains uncertain.

Different competent people may disagree.

The manager decides whose judgement to trust.

Clubs can make understandable mistakes.
```

## Media intelligence

```text
Journalism is evaluated by the actual claim,
not merely the eventual outcome.

Truth
framing
intent
source manipulation
public perception
and final outcome
are separate.

Credibility is contextual.

Reach is not credibility.

Belief is not sharing.

Clubs are made of people, not one opinion.

Media relationships are directional,
persistent,
causal,
and remembered.

Journalists, players, managers, agents,
executives, outlets, and other media
can develop alliances, grudges, access,
respect, rivalry, and long-term conflict
through real history.
```

These decisions are locked.

---

# Locked Addendum: Transfers, Squad Planning, Adaptation, and Contracts

> **Status: LOCKED**
>
> This addendum records the additional decisions explicitly locked after the original document was created. It is part of the same design authority and should be read together with the earlier sections.

# 3. Transfer Economics, Recruitment Decisions, Negotiation, and Organisational Risk

## 3.1 Core rule: transfers are organisational decisions, not simple upgrades

A transfer must not behave as:

```text
need player
→ find better player
→ buy player
```

A real club decision emerges from several interacting layers:

```text
HARD FEASIBILITY
+ FOOTBALL VALUE
+ STRATEGIC VALUE
+ ORGANISATIONAL POLITICS
+ FINANCE
+ PLAYER / AGENT WILLINGNESS
+ REPLACEMENT REALITY
+ INFORMATION UNCERTAINTY
+ DYNAMIC RISK APPETITE
+ SUPPORTER / MEDIA / DRESSING-ROOM CONTEXT
```

These factors do **not** all have equal weight. They should not behave like a checklist where every concern gets one vote.

## 3.2 Hard feasibility can genuinely block a deal

Examples include:

- transfer fee cannot be financed
- wage package exceeds real constraints
- registration limits make the signing impossible
- work-permit or legal rules fail
- player refuses the move
- agent refuses the package
- medical risk exceeds club tolerance
- selling club refuses
- deadline prevents completion
- competition rules prevent registration

Hard feasibility differs from soft resistance.

## 3.3 Football value normally carries the greatest soft weight

The club should strongly consider:

- perceived quality improvement
- tactical fit
- scarcity of the profile
- urgency of the position
- current alternatives
- injury crisis
- title/relegation pressure
- expected role
- manager need
- ability to solve an immediate football problem

A marginal upgrade can be rejected because of secondary concerns.

A transformational perceived upgrade can overwhelm many secondary concerns.

Example:

```text
Current player:
estimated level 72

Target:
estimated level 90+
perfect tactical fit
affordable
wants to join
```

The club may deliberately accept:

- supporter hostility
- agent difficulty
- media friction
- unusual wage demand
- dressing-room resistance
- adaptation risk
- resale uncertainty

because the football upside is enormous.

## 3.4 Soft resistance is not the same as a veto

If a senior player opposes a signing, the result should not automatically be:

```text
senior player objects
→ transfer cancelled
```

The club can sign the player anyway.

The objection can then survive as consequence:

```text
signing completed
→ senior player remains unhappy
→ clique tension
→ manager intervention
→ media questions
→ relationship effects
```

The same applies to:

- supporters
- media
- staff disagreement
- agent relationships
- wage hierarchy
- personality concerns

A strong football case can overpower soft resistance without erasing it.

## 3.5 "Clear upgrade" is perception-based

This inherits the anti-omniscience rules from the earlier locked section.

The club must not compare hidden truth directly:

```text
Target true CA = 91
Current player true CA = 72
```

The club should instead hold beliefs such as:

```text
Current player:
estimated 70-75
confidence very high

Target:
estimated 84-92
confidence medium-high
```

The club can therefore make a rational decision that later proves wrong.

## 3.6 Club governance determines whose opinion matters

Do not average every internal opinion into one club score.

Different clubs can have different power structures:

- manager-led
- sporting-director-led
- owner-led
- recruitment-committee-led
- data-led
- hybrid structures

Example:

```text
Manager: strongly wants player
Sporting director: lukewarm
Recruitment head: prefers alternative
Owner: loves commercial upside
Captain: dislikes target
Supporters: divided
```

The outcome depends on institutional power, not a simple average.

## 3.7 Internal disagreement persists after the decision

If the director forces through a player the manager opposed, that history should remain.

If the player succeeds:

```text
director credibility ↑
manager may revise view
board trusts recruitment team more
```

If the player fails:

```text
manager gains leverage
director credibility ↓
future recruitment authority may change
```

Important transfers should leave causal institutional memory.

## 3.8 Public market value is not the transaction price

Separate:

- public market estimate
- buyer's fair-value belief
- buyer's maximum willingness-to-pay
- seller's internal valuation
- seller's actual minimum
- competing buyer valuation

Example:

```text
Public estimate: £35m

Buyer A fair value: £42m
Buyer A maximum: £52m

Buyer B fair value: £28m
Buyer B maximum: £31m

Seller internal value: £50m
Seller minimum in current situation: £46m
```

All can be reasonable simultaneously.

## 3.9 Fair value and willingness-to-pay are distinct

A club may believe a player is worth £40m but pay £52m because:

- profile is scarce
- alternatives failed
- need is urgent
- manager insists
- title window is open
- deadline pressure is extreme

Another club may believe the player is worth £40m but refuse above £30m because:

- it has alternatives
- budget is needed elsewhere
- adaptation risk is high
- need is not urgent

## 3.10 Selling decisions depend on more than fee

The seller should consider:

- tactical importance
- replacement availability
- contract length
- player desire
- board desire
- finances
- squad depth
- academy options
- deadline timing
- current sporting objectives
- future value
- supporter importance
- homegrown/registration value
- competing buyers

Two similarly valued players can therefore have radically different sale thresholds.

## 3.11 Replacement chains are allowed

Transfers can depend on each other:

```text
Club A wants Player X
→ seller will only sell after replacement
→ seller targets Player Y
→ Player Y's club needs its own replacement
```

This can create:

```text
deal agreed
→ replacement collapses
→ seller pulls out
```

or:

```text
replacement signs
→ seller becomes flexible
→ original transfer accelerates
```

## 3.12 Alternative targets create bargaining leverage

A buyer with three strong alternatives can walk away.

A buyer whose alternatives disappeared may knowingly overpay.

The seller may or may not know how desperate the buyer has become.

## 3.13 Negotiation operates under information asymmetry

The buyer should not know the seller's true minimum.

The seller should not know the buyer's true maximum.

The agent should not automatically know either.

Example:

```text
Buyer belief:
seller probably accepts £44m-50m

Seller belief:
buyer probably reaches £55m

Reality:
seller would accept £42m
buyer maximum is £48m
```

A mutually acceptable deal can still fail because both sides misread the other.

## 3.14 Bluffing and signalling are allowed

Actors can:

- exaggerate rival interest
- claim budget exhaustion
- leak interest
- threaten to walk
- delay responses
- set artificial urgency
- use alternative targets as leverage

Success depends on:

- negotiator skill
- reputation
- credibility
- relationship history
- information quality
- actual alternatives
- deadline pressure

This should not become a visible fake poker minigame.

## 3.15 Agents have strategy, competence, relationships, and incentives

Agents can:

- create competing interest
- brief journalists
- pressure the current club
- delay decisions
- seek release clauses
- prioritise role over wage
- prioritise signing bonus
- maximise agent fees
- protect future mobility
- steer toward certain leagues or clubs

Agents can also make mistakes:

- overplay leverage
- lose a move
- wait for an offer that never arrives
- damage a director relationship
- accept too early

## 3.16 Player decisions consider the whole move

A player may care about:

- base wage
- guaranteed total money
- bonuses
- role
- expected minutes
- tactical fit
- manager trust
- manager reputation
- club ambition
- league quality
- continental competition
- city
- climate
- language
- family
- relocation
- teammates
- supporter environment
- media environment
- future mobility
- release clauses
- contract security
- agent advice
- relationship with current club

Weights depend on personality and situation, not just age.

## 3.17 Supporters matter contextually

Supporter influence depends on:

- ownership sensitivity
- club culture
- supporter power
- rivalry
- player history
- controversy
- football value
- current supporter mood

Fans do not receive a magical transfer veto.

## 3.18 Media context can create risk

A target may already have poor relationships with influential local media.

That can increase:

- scrutiny
- communication risk
- pressure
- likelihood of controversy

But:

```text
media dislikes player
≠ automatic rejection
```

The club can still decide the football upside justifies it.

## 3.19 Senior players can oppose a signing

Current players may react because of:

- role competition
- wage hierarchy
- captaincy
- rivalry history
- personality
- tactical implications
- broken promises

Again, this creates friction, not necessarily a veto.

## 3.20 Sponsor and commercial context can affect risk appetite

Sponsors should not usually choose players directly.

But sponsor/commercial pressure can affect the club's willingness to take risks.

Examples:

- new sponsor wants greater visibility
- ownership wants a marquee arrival
- commercial department sees market-expansion value
- investment creates pressure to show ambition

This can make the club more willing to accept:

- high fee
- high wage
- adaptation uncertainty
- lower resale certainty
- media risk
- personality risk

## 3.21 Club risk appetite is dynamic

Risk appetite is not a permanent slider.

It changes with:

- new ownership
- investment
- sponsor changes
- cash reserves
- debt
- recent recruitment success/failure
- director tenure
- manager tenure
- board pressure
- supporter mood
- media scrutiny
- league position
- title challenge
- relegation danger
- recent trophies
- transfer-window timing
- alternatives
- future financial constraints

Example:

```text
new owner
+ large investment
+ Champions League qualification
→ club becomes aggressive
```

Another:

```text
two expensive flops
+ board scrutiny
+ supporter anger
→ club becomes conservative
```

## 3.22 Individual decision-makers also have risk tendencies

The club's state is not identical to each person's personality.

Example:

```text
Sporting director:
aggressive, change-oriented

Owner:
cautious, commercially focused

Manager:
wants immediate proven quality
```

Their conflict can shape the final decision.

## 3.23 Distinguish missed risk from accepted risk

This is mandatory.

Case A:

```text
club never recognised adaptation risk
→ player fails
```

Case B:

```text
club recognised adaptation risk
→ chose to accept it
→ player fails
```

Those are not the same decision.

Outcome alone must not determine whether the original decision was good.

## 3.24 Important deals should retain causal memory

Where practical, preserve:

- who recommended the player
- who opposed
- who approved
- what evidence existed
- what risks were known
- what role was expected
- what alternatives existed
- what price range was believed acceptable
- why the club proceeded

This should later feed:

- board judgement
- director reputation
- manager relationships
- media narratives
- supporter narratives
- future club risk appetite


## 3.25 Public market value is not the transfer price

A public or estimated market value is contextual information. It is **not** a mandatory anchor that determines an offer.

Keep distinct:

```text
PUBLIC / ESTIMATED MARKET VALUE
SELLER'S INTERNAL VALUATION
SELLER'S CURRENT RESERVATION PRICE
BUYER'S FOOTBALL VALUATION
BUYER'S OPENING BID
BUYER'S MAXIMUM WILLINGNESS TO PAY
PLAYER'S OWN WILLINGNESS TO MOVE
EVENTUAL AGREED FEE
```

These values can differ dramatically.

A club may discover a player in an under-scouted, low-wage, or financially weaker football market and legitimately obtain him for a fee far below what he may later be worth in another context.

Example:

```text
Local club originally paid: ₹2 lakh

Foreign club scouts player
↓
rates player highly
↓
seller would currently accept ₹10 lakh
↓
player wants the move
↓
little known competition

Result:
₹10 lakh can be a completely rational transfer fee.
```

The simulation must not "correct" the deal upward merely because a public valuation model believes the player is worth more.

Scouting asymmetry is allowed to create genuine bargains.

## 3.26 Football markets are economically asymmetric

The same player can be priced very differently depending on:

- current league
- selling club finances
- wage level
- contract duration
- club ambition
- player pressure
- local transfer norms
- international exposure
- number of credible buyers
- replacement difficulty
- registration value
- timing
- seller cash need
- buyer urgency

A player's underlying football ability can remain almost unchanged while the transfer environment around him changes radically.

Example:

```text
Unknown player in low-exposure league
→ cheap wages
→ little buyer competition
→ seller has limited leverage

Two years later:
same player now established in stronger league
→ higher wage
→ much more evidence
→ bigger reputation
→ several buyers
→ stronger seller
→ fee can rise by orders of magnitude
```

That does not mean his ability rose by the same factor.

## 3.27 Information can change negotiation value overnight

A club's reservation price and bid strategy should react to **credible new information**.

Example:

```text
Seller yesterday:
would accept ₹10 lakh

Today:
credible information arrives that another club
is preparing a ₹60 lakh bid

Seller may now:
- wait
- reject ₹10 lakh
- ask ₹60-80 lakh
- demand stronger add-ons
- attempt an auction
```

The first buyer may then:

- walk away
- hold position
- question the credibility of the rival bid
- accelerate an offer
- increase to ₹80 lakh
- restructure the package
- pursue an alternative target

No action is compulsory.

## 3.28 Rival-bid information has provenance and uncertainty

"The other club is preparing ₹60 lakh" is not automatically true.

The information may come from:

- agent
- journalist
- intermediary
- scout contact
- selling director
- board member
- player representative
- leaked club employee
- another buyer

The receiving club should reason about:

```text
How credible is the source?
Why are we being told?
Who benefits if we believe it?
Is this formal interest or exploratory interest?
Could it be an agent or seller bluff?
How much do we care if it is real?
```

This plugs directly into the media, relationship, and information systems.

## 3.29 A seller can knowingly gamble on future value

A selling club may reject a currently profitable offer because it believes future upside is greater.

It may also accept a low offer despite believing the player is worth more because:

- cash is urgently required
- contract is close to expiry
- player wants out
- wage burden matters
- relationship has broken down
- replacement is already available
- registration pressure exists
- board wants certainty

The seller's decision must therefore be contextual, not a lookup against "market value."

## 3.30 Transfer fees are discovered through negotiation

A transfer fee emerges from:

```text
seller reservation price
+ buyer valuation
+ player willingness
+ contract leverage
+ local economics
+ scouting asymmetry
+ competing interest
+ information quality
+ urgency
+ bargaining skill
+ organisational strategy
```

Historical fee, public market value, reputation, and statistical production can all be evidence, but none is the transaction oracle.

A cheap transfer that later becomes enormously valuable is not inherently a simulation error. It can be the natural consequence of superior scouting, weak seller leverage, low competition, or changed future circumstances.

---

# 4. Squad Planning, Tactical Fit, Integration, and Adaptation

## 4.1 Squad planning is not future-CA arithmetic

The club should not know exact future numbers.

It should maintain uncertain beliefs about:

- current quality
- decline risk
- development
- depth
- tactical fit
- contracts
- likely departures
- injuries
- academy readiness
- replacement difficulty

## 4.2 Maintain a living squad map

Useful dimensions include:

```text
CURRENT NEED
FUTURE NEED
SUCCESSION RISK
DEPTH
QUALITY
AGE
CONTRACT
TACTICAL FIT
HOMEGROWN / REGISTRATION
DEVELOPMENT PATHWAY
EXPECTED DEPARTURES
INJURY / AVAILABILITY
```

## 4.3 Plan across multiple time horizons

Conceptual horizons:

```text
NOW
0-6 months

SHORT TERM
next season

MEDIUM TERM
2-3 seasons

LONGER TERM
3-5 seasons
```

Confidence should decline with distance.

Long-range planning should be scenario-based.

## 4.4 Use scenarios instead of fake certainty

Example:

```text
Scenario A:
starter remains elite

Scenario B:
noticeable decline begins

Scenario C:
injury or contract issue changes plan
```

A club may therefore monitor successors without immediately buying one.

## 4.5 Contract state creates future needs before departure

If a key player has 12 months left and renewal probability is falling, the squad plan should react before the player actually leaves.

## 4.6 Player intentions matter

The club can consider what it believes about:

- satisfaction
- desire for larger role
- Champions League ambition
- agent activity
- free-agency interest
- family concerns
- manager relationship

## 4.7 Manager and director can disagree about squad construction

Examples:

```text
Manager:
wants experienced centre-back now

Sporting director:
wants academy pathway protected

Recruitment head:
academy players are not ready

Finance:
cannot fund every priority
```

Governance determines who carries power.

## 4.8 Squad-building philosophies differ

Possible philosophies:

- small squad
- large rotation squad
- youth-first
- veteran-heavy
- athletic
- technical
- versatile
- specialist
- aggressive turnover
- continuity
- buy-young/develop/sell
- win-now

A manager change can therefore change a player's strategic status without changing ability.

## 4.9 Recruitment usually begins with a defined football problem

Example:

```text
NEED:
left-footed centre-back

Reason:
starter ageing
backup likely leaving
manager builds through left side

Desired:
starter quality
progressive passing
high-line suitability
age preference 21-27

Constraints:
budget
wage structure
registration
academy pathway
adaptation
```

## 4.10 Opportunistic recruitment remains valid

Sometimes an exceptional opportunity appears.

A club can deviate from its plan because a normally unobtainable player suddenly becomes available at exceptional value.

Distinguish:

```text
planned recruitment
```

from:

```text
opportunity recruitment
```

## 4.11 Tactical fit matters before signing

The club should ask:

> Is this player good for what we intend to do?

Relevant dimensions may include:

- position
- role
- pressing
- line height
- build-up
- tempo
- transition style
- aerial requirements
- physical demands
- off-ball movement
- set pieces
- tactical intelligence
- versatility

## 4.12 Signing does not mean instant full effectiveness

This is locked:

```text
SIGNED
≠
AVAILABLE
≠
ACCLIMATISED
≠
TACTICALLY INTEGRATED
≠
PERFORMING AT EXPECTED LEVEL
```

A medically available player may still be far from fully integrated.

## 4.13 Adaptation is multi-dimensional

### Physical/environmental

- temperature
- humidity
- altitude
- daylight pattern
- pitch/environment
- training load
- league physicality
- travel demands

### Body clock/routine

- timezone change
- sleep
- meals
- daily routine
- training time
- travel fatigue

### Football adaptation

- tactical system
- role
- pressing triggers
- rotations
- rest defence
- set pieces
- teammate movements
- transition expectations
- league pace
- refereeing style

### Social adaptation

- language
- communication
- culture
- housing
- family relocation
- teammates
- social support

### Mental adaptation

- confidence
- expectation
- homesickness
- acceptance
- pressure
- supporter reception
- media attention

These channels should not share one generic timer.

## 4.14 Environmental distance is contextual, not nationality hardcoding

Do not implement:

```text
India → Russia = bad
```

as a nationality rule.

Instead evaluate real differences:

- climate
- timezone
- altitude
- language
- culture
- football style
- travel pattern

History matters.

An Indian player with years in Norway may adapt to Russia far more easily than an Indian player moving abroad for the first time.

## 4.15 Previous experience reduces relevant uncertainty

Useful prior experience includes:

- living abroad
- similar climate
- local language
- similar tactical system
- same manager
- same staff
- familiar teammates
- previous league
- extensive international travel

## 4.16 Player traits affect adaptation speed

Possible influences:

- adaptability
- professionalism
- resilience
- social confidence
- experience
- age
- languages
- family situation
- relocation history
- physical conditioning
- personality
- cultural familiarity

Two players making the same move should not adapt identically.

## 4.17 Club support matters

Possible support:

- housing
- family relocation
- language tutors
- nutrition
- personal liaison
- sports psychology
- conditioning plan
- medical monitoring
- cultural support
- teammate mentoring
- gradual training plan

A well-run club can reduce transition friction.

## 4.18 Tactical familiarity has its own timeline

A player can be physically ready but tactically slow.

They may need to learn:

- pressing cues
- width/inversion timing
- overlap timing
- build-up positions
- defensive rotations
- rest defence
- transitions
- set pieces

A world-class player can initially underperform because they are thinking instead of reacting automatically.

## 4.19 Clubs should estimate time-to-usefulness

Recruitment reports should include beliefs such as:

```text
Football adaptation risk: low
Climate adjustment risk: moderate
Language risk: high
Family relocation: uncertain
Physical transition: moderate

Immediate availability:
high

Immediate full effectiveness:
low-medium

Confidence:
medium
```

## 4.20 Managers choose how aggressively to integrate the player

Possible strategies:

- start immediately
- gradual starts
- substitute appearances
- training-only period
- reduced minutes
- simplified role
- conditioning first

The choice depends on:

- urgency
- quality advantage
- match importance
- tactical readiness
- physical state
- manager risk tolerance
- current alternatives

## 4.21 Early use can accelerate or damage adaptation

Positive loop:

```text
plays early
→ performs well
→ confidence rises
→ supporters embrace player
→ teammates trust player
→ integration accelerates
```

Negative loop:

```text
poor sleep
+ unfamiliar intensity
+ heavy minutes
→ recovery worsens
→ training quality falls
→ fatigue rises
→ confidence suffers
→ injury risk may rise
```

These are pressures, not guaranteed scripts.

## 4.22 Planning should include time until useful

A club may prefer:

```text
Player A:
slightly lower ceiling
ready immediately
```

over:

```text
Player B:
higher ceiling
major adaptation period
```

when the need is urgent.

A rebuilding club may prefer the opposite.

## 4.23 Failed planning should create history

Examples:

```text
academy prospect overestimated
→ club skips replacement
→ prospect not ready
→ squad weakness appears
```

```text
expensive veteran signed
→ youngster blocked
→ youngster leaves
→ youngster becomes star elsewhere
```

```text
club expects renewal
→ talks collapse
→ alternatives already gone
→ panic signing
```

These decisions should affect future judgement and reputation.

---

# 5. Contract Architecture, Offer Variation, Clauses, and Options

## 5.1 Clubs should not all offer the same contract with a different wage

Two clubs can value the same player similarly and still offer very different packages.

Example:

```text
Club A
£65k/week
4 years
Regular Starter
£2m signing bonus
appearance bonuses
Champions League bonus
club option +1
```

```text
Club B
£82k/week
3 years
Important Player
smaller signing fee
large performance bonuses
release clause
player option
```

```text
Club C
£58k/week
5 years
Star Player
large loyalty bonus
no release clause
annual wage rises
```

All three can be rational.

## 5.2 Contract structure follows club strategy

Offer structure can depend on:

- finances
- cash flow
- wage budget
- wage hierarchy
- perceived player value
- intended role
- age
- adaptation uncertainty
- injury uncertainty
- resale strategy
- owner policy
- sporting-director policy
- manager importance
- league norms
- legal/tax environment
- competing offers
- player leverage
- agent leverage
- deadline urgency

Conceptually:

```text
CONTRACT OFFER
=
club finances
+ wage structure
+ perceived value
+ intended role
+ risk
+ player leverage
+ adaptation uncertainty
+ club strategy
+ league/legal context
+ agent strategy
+ competing offers
```

## 5.3 Negotiate the whole package

Possible dimensions include:

- base wage
- signing-on fee
- loyalty bonus
- appearance bonus
- start bonus
- goal bonus
- assist bonus
- clean-sheet bonus
- title bonus
- promotion bonus
- continental qualification bonus
- international-cap bonus
- annual wage rise
- relegation wage reduction
- release clause
- relegation release clause
- club option
- player option
- mutual option
- automatic extension trigger
- contract length
- squad status
- playing-time promise
- positional/role promise
- agent fee
- image/commercial rights where appropriate
- other legally valid clauses

Not every club or jurisdiction should use every clause.

## 5.4 Negotiation allows trade-offs

Example:

```text
Player:
will accept lower wage
if club grants Important Player status
and reasonable release clause
```

Club:

```text
no release clause
but larger signing bonus
```

Agent:

```text
then shorter contract
or second-year wage rise
or player option
```

This creates real package negotiation rather than a wage auction.

## 5.5 Squad role is a material promise

If the club sees the player as a star, the package should reflect that through:

- wage
- minutes
- status
- leadership expectation
- contract length
- bonuses
- public messaging
- tactical promises

Broken promises should have consequences.

## 5.6 Wage hierarchy matters

A club may have a structure such as:

```text
elite stars:      £150k-180k
regular starters: £80k-120k
rotation:         £40k-70k
youngsters:       £10k-30k
```

A newcomer on £200k can trigger:

```text
senior players notice
→ agents demand raises
→ renewals become harder
→ wage inflation
→ possible resentment
```

The club can still deliberately break the structure.

## 5.7 Investment can change contract policy

Takeover, sponsor growth, promotion, Champions League qualification, or a change in strategic ambition can make a club abandon an old wage ceiling.

## 5.8 Contract length reflects risk and strategy

Young high-upside player:

```text
long deal
club option
value protection
```

Older player:

```text
shorter guarantee
club option
appearance triggers
```

High adaptation uncertainty:

```text
shorter initial term
option
more variable compensation
```

Desperate elite target:

```text
long guarantee
high wage
large signing fee
```

## 5.9 Clubs can shift risk between guaranteed and variable pay

Risk-averse club:

```text
lower base
+ stronger bonuses
```

Security-seeking player:

```text
higher guaranteed wage
+ fewer conditional bonuses
```

## 5.10 Player priorities vary

Players can prioritise:

- guaranteed money
- maximum wage
- security
- playing time
- Champions League
- club prestige
- release clause
- future flexibility
- family stability
- city
- language
- title opportunity
- development path

A young ambitious player may prefer lower wage plus pathway.

An older player may prefer guaranteed years and a large signing bonus.

## 5.11 Adaptation risk can affect contract terms

Player side may demand compensation for:

- relocation
- climate
- family disruption
- language
- league uncertainty
- lifestyle change

Club side may prefer:

- option years
- variable compensation
- shorter guarantee
- performance incentives

## 5.12 Same gross wage in different countries is not automatically equivalent

Player utility may consider:

- tax
- cost of living
- currency stability
- housing
- family costs
- bonus treatment
- image rights
- league prestige
- relocation burden

The simulation does not need microscopic tax law everywhere, but it should preserve the principle.

## 5.13 Agents optimise different parts of the package

An agent may prioritise:

- player wage
- signing fee
- agent fee
- release clause
- length
- future mobility
- role
- bonus structure
- option control

Agent incentives can sometimes diverge from player preferences.

## 5.14 Options have ownership and strategic meaning

### Club option

Controlled by the club.

Useful for:

- age uncertainty
- injury uncertainty
- protecting value
- reducing commitment

### Player option

Controlled by the player.

Useful for:

- security
- leverage
- flexibility

### Mutual option

Requires agreement or an agreed trigger structure.

### Automatic extension

Can be triggered by legally valid conditions such as:

- appearances
- starts
- promotion
- international caps
- team achievements

The party controlling the option materially changes its value.

## 5.15 Release clauses are strategic

A release clause can:

- help convince a player to join
- protect future mobility
- create future risk for the club
- compensate for lower salary
- vary by relegation/competition if legally valid
- become a future source of conflict

A club may refuse a clause and compensate elsewhere.

## 5.16 Role and playing-time promises become future state

If the club promises:

```text
Important Player
```

but later treats the player as fringe, consequences can include:

- dissatisfaction
- agent pressure
- manager conversations
- transfer request
- leak to media
- trust loss
- dressing-room reaction

The promise must be remembered as part of the signing history.

## 5.17 Clubs can overcommit

A desperate director may offer:

```text
five years
huge guaranteed wage
large signing fee
```

If the player fails, the contract can damage the club for years through:

- poor saleability
- wage-budget pressure
- reduced recruitment flexibility
- board criticism
- supporter anger
- director reputation damage

## 5.18 Contract quality is not identical to outcome

A sensible contract can turn bad because of unexpected injury.

A reckless contract can look brilliant because the player unexpectedly becomes elite.

Historical judgement should consider what was known at the time.

## 5.19 Important contracts should retain causal memory

Preserve enough information to reconstruct:

- what the club believed the player was worth
- intended role
- known risks
- competing offers
- negotiation alternatives
- who approved exceptional terms
- which clauses were concessions
- what player/agent prioritised
- what promises were made

That history should feed:

- board judgement
- media
- supporters
- player-agent relationships
- manager/director credibility
- future negotiation policy

---

# Locked Summary for Sections 3-5

```text
TRANSFERS

Transfers are weighted organisational decisions,
not simple value comparisons.

Football value normally dominates soft factors,
but affordability, registration, governance,
replacement reality, player willingness, agents,
supporters, media, dressing-room context,
and dynamic risk appetite can matter.

A clear perceived upgrade can overpower soft objections
without erasing their later consequences.

Clubs can knowingly take risks.

Buyer value, seller value, public value,
fair value, and willingness-to-pay are separate.

Negotiation happens under uncertainty.
```

```text
SQUAD PLANNING

Squad planning is multi-horizon and uncertain.

Clubs plan around:
quality
depth
contracts
age
departures
tactical fit
academy pathways
registration
injuries
finances
scenario risk
time-to-usefulness.

A signing is not instantly fully effective.
```

```text
ADAPTATION

SIGNED
≠ AVAILABLE
≠ ACCLIMATISED
≠ TACTICALLY INTEGRATED
≠ FULLY EFFECTIVE

Climate
timezone
language
culture
family
football style
tactical familiarity
confidence
player adaptability
prior experience
and club support
can all affect transition.

Different adaptation dimensions move on different timelines.
```

```text
CONTRACTS

Contracts are packages, not wage numbers.

Different clubs can rationally offer different:
wages
lengths
bonuses
roles
promises
release clauses
options
and risk-sharing structures.

Wage hierarchy matters.
Options have ownership.
Promises become future state.
Exceptional contracts create downstream consequences.

Contract quality is judged from what was known
when the decision was made, not only the outcome.
```

These additional decisions are locked.

---

# 6. Social Opinion, Public Attention, Virality, Brand Value, and Cultural Narrative

> **Status: LOCKED**
>
> The social layer must simulate how people actually form, express, revise, exaggerate, joke about, weaponise, and remember opinions around footballers. It must not collapse public response into one global sentiment number.

## 6.1 Social opinion is multidimensional

A person can simultaneously think:

- a player is excellent at football
- a player is in poor form
- a player is tactically useful
- a player is tactically overrated
- a player is likeable
- a player is annoying
- a player is trustworthy
- a player is disloyal
- a player is attractive
- a player is unattractive
- a player is good value
- a player is terrible value
- a player represents the club well
- a player does not feel like "one of us"

Therefore one universal opinion score is insufficient.

Useful conceptual dimensions may include:

```text
FOOTBALL EVALUATION
FORM EVALUATION
TACTICAL APPROVAL
PERSONAL AFFECTION
TRUST
PROFESSIONAL RESPECT
LOYALTY PERCEPTION
EFFORT PERCEPTION
IDENTIFICATION / "ONE OF US"
VALUE-FOR-MONEY PERCEPTION
EXPECTATION
RESENTMENT / GRIEVANCE
AESTHETIC / ATTRACTION RESPONSE
CHARISMA RESPONSE
BRAND / CELEBRITY INTEREST
```

Not every dimension needs to be visible or stored identically for every actor.

## 6.2 Different events update different opinion dimensions

A hat-trick may strongly improve:

- current-form evaluation
- football evaluation
- confidence in the player

while barely changing:

- trust
- loyalty perception
- personal affection
- long-running grievance

A new contract may improve loyalty perception.

A lie that is later exposed may damage trust.

A viral interview may improve affection or celebrity interest without changing football evaluation.

The system should not treat all positive events as "+opinion".

## 6.3 Important memories persist and can reactivate

Social memory should preserve important events with context such as:

- event
- interpretation
- confidence
- emotional importance
- age of memory
- whether later evidence contradicted it

Old memories should be able to reactivate.

Example:

```text
2027:
player publicly criticises supporters

2030:
new supporter dispute happens

→ old event resurfaces
→ "Here we go again"
```

The memory may fade in intensity without disappearing arbitrarily.

## 6.4 People can disagree about why something happened

The public rarely has full information.

If a player is benched, different accounts may believe:

- tactical decision
- injury
- attitude problem
- rotation
- punishment
- transfer issue
- manager incompetence

Those are beliefs, not necessarily facts.

Confidence in those beliefs should vary and update as new information arrives.

## 6.5 Belief confidence matters

A supporter can hold:

```text
"Maxx may want to leave"
confidence: low
```

without immediately converting that into a firm loyalty judgement.

As evidence changes, confidence changes.

Updates can depend on:

- trust in player
- trust in journalist
- prior beliefs
- club allegiance
- rivalry
- emotional preference
- knowledge
- credulity

## 6.6 Tribalism changes interpretation, not objective reality

The same incident can produce different interpretations:

```text
Home supporter:
"Hard challenge, never a red."

Rival supporter:
"Dirty as always."

Neutral analyst:
"Late and probably a red."
```

The underlying event stays the same.

Interpretation changes because of:

- allegiance
- rivalry
- personality
- knowledge
- history
- emotion

## 6.7 People can hold apparently contradictory opinions

Examples:

```text
likes player personally
but thinks player is overrated
```

```text
dislikes player personally
but thinks player is world-class
```

```text
thinks player is excellent
but hates the transfer fee
```

```text
thinks player is mediocre
but loves him because he came through the academy
```

The architecture must support these combinations.

## 6.8 Group identity changes tolerance and expectation

Players can represent different things:

- academy graduate
- local player
- foreign superstar
- former rival
- captain
- record signing
- bargain signing
- cult hero
- club legend's relative
- national icon
- expensive outsider

These identities change how supporters interpret the same performance.

## 6.9 Expectation is first-class

Public reaction should often be based on:

```text
observed performance
versus
expected performance
```

A mediocre match from a club-record signing may disappoint.

The same match from an academy debutant may excite supporters.

Expectation can be influenced by:

- transfer fee
- wage
- reputation
- role promise
- media hype
- age
- previous form
- club status
- recent viral attention

## 6.10 Price and wages can become social information

If a player becomes the highest-paid player at the club:

```text
expectation rises
media scrutiny rises
teammates notice
agents notice
supporters compare value
```

This can create future narratives around:

- value
- entitlement
- performance
- dressing-room hierarchy
- renewal demands

## 6.11 Opinion differs by audience

Do not assume one "public opinion."

Different audiences can include:

- own supporters
- rival supporters
- neutral supporters
- local supporters
- national supporters
- international supporters
- matchgoing fans
- online-only fans
- ultras
- casual fans
- stats accounts
- tactical accounts
- academy-focused fans
- journalists
- former players
- celebrity/fashion audiences

The same player can have radically different reputations among these groups.

## 6.12 Social opinion must affect behaviour

Opinion should change what people do.

Examples:

A distrustful account may:

- believe negative rumours more easily
- bring up old incidents
- interpret ambiguity negatively
- resist apologies

A loyal supporter may:

- defend the player
- give benefit of doubt
- challenge criticism
- share positive content

But strong evidence should still be capable of changing views.

## 6.13 People can revise, defend, hide, or deny old opinions

A persistent account that once posted:

> "Sell him."

can later react in different ways if the player becomes elite:

- admit error
- delete old post
- laugh at themselves
- pretend it never happened
- defend the original context
- double down

Other users can quote the old post.

This is an important use of persistent social history.

## 6.14 Collective sentiment should use layered simulation

Do not fully simulate millions of accounts.

Use:

```text
population-level aggregates
+
persistent representative accounts
+
important/high-reach accounts
+
materialised relevant threads
```

Aggregate sentiment itself can be multidimensional, for example:

- football approval
- manager approval
- transfer satisfaction
- trust
- optimism
- anger
- excitement
- commercial interest

## 6.15 Players perceive public opinion imperfectly

The player should not read:

```text
Fans like you: 67%
```

as objective truth.

The player experiences:

- posts they see
- chants
- cheers
- boos
- media questions
- teammate comments
- agent feedback
- direct messages
- stadium atmosphere

A player can therefore overestimate hostility because negative posts are louder, or underestimate criticism because they avoid social media.

```text
actual public state
≠
player perception of public state
```

## 6.16 Appearance and attraction can affect public response

Some people may follow or like a player because they find the player attractive.

Others may make negative appearance-based comments.

The simulation itself should not present attractiveness as an objective moral truth.

Instead, individual accounts can have their own aesthetic response.

Possible audience reactions include:

- "he looks good"
- fashion interest
- physique interest
- thirst comments
- fan edits
- attraction-driven following
- cruel or mocking appearance comments
- counter-reactions defending the player

Appearance discourse can be positive, neutral, playful, superficial, cruel, or hostile.

## 6.17 Attractiveness is audience-specific

Do not implement:

```text
Handsome = 92
therefore everyone agrees
```

Instead distinguish underlying presentation traits and audience response.

Possible influences include:

- appearance
- physique
- grooming
- fashion
- style
- camera presence
- charisma
- confidence
- cultural preference
- audience taste
- age group
- social context

One audience can find a player very attractive while another barely cares.

## 6.18 Football reputation, fame, attention, and commercial appeal are separate

Keep distinct concepts such as:

```text
FOOTBALL REPUTATION
PUBLIC FAME
SOCIAL ATTENTION
COMMERCIAL APPEAL
SOCIAL REACH
CULTURAL RELEVANCE
```

A player can be:

- world-class but commercially quiet
- good but enormously famous
- average but a cult celebrity
- elite but widely disliked
- famous before most people know whether he is good

These combinations are desirable.

## 6.19 Brand value emerges from many sources

Commercial appeal can be influenced by:

- football success
- attractiveness
- charisma
- style
- nationality
- geographic audience
- club
- personality
- interviews
- viral moments
- controversies
- social activity
- sponsor history
- audience demographics
- audience engagement

Do not collapse this into one fixed "marketability" attribute.

## 6.20 Virality creates attention shocks

A reel, clip, goal, interview, photo, celebration, joke, meme, or controversy can suddenly explode.

Example:

```text
training reel
+ good timing
+ attractive visual
+ trending audio
+ teammate moment
+ large repost account

→ views accelerate
→ new audiences discover player
```

Consequences can include:

- follower growth
- name recognition
- sponsor interest
- media coverage
- international awareness
- merchandise demand
- fan-edit growth

without any change in football ability.

## 6.21 Virality has different causes

Useful categories can include:

### Football virality

- wondergoal
- outrageous skill
- save
- terrible miss
- tackle
- celebration

### Personality virality

- funny interview
- teammate interaction
- reaction
- press-conference moment

### Aesthetic virality

- photoshoot
- outfit
- appearance
- physique
- hairstyle
- training clip

### Controversy virality

- argument
- insult
- red card
- leaked clip
- public dispute

### Emotional virality

- crying after final
- comeback
- family moment
- injury return

### Meme virality

- facial expression
- celebration
- fan edit
- accidental clip
- recurring joke

Different causes attract different audiences.

## 6.22 Viral attention can migrate outside football audiences

A player can move from:

```text
football audience
→ fashion audience
→ general social media
→ celebrity pages
→ fan-edit communities
```

People may know the player's face and name without understanding football.

This can substantially increase commercial value.

## 6.23 Follower quality matters, not only follower count

Commercial demand can care about:

- geography
- age demographic
- engagement
- brand safety
- purchasing power
- interest category
- audience loyalty

Ten million highly engaged young international followers can be commercially different from ten million passive followers.

## 6.24 Brand value can matter in transfers without replacing football value

A club may compare:

```text
Player A:
slightly stronger football fit
lower commercial reach

Player B:
almost as strong
massive international audience
sponsor appeal
target-market value
```

At some clubs, commercial value may legitimately influence the decision.

At others, sporting leadership may barely care.

A huge following must not automatically compensate for clearly inadequate football level.

## 6.25 Opinion volatility differs between people

Some accounts are highly reactive.

Others require sustained evidence.

Useful hidden tendencies may include:

- recency bias
- patience
- sample-size sensitivity
- emotionality
- trend-following
- narrative susceptibility
- contrarianism

After one huge match:

```text
Account A:
"world class"

Account B:
small positive update

Account C:
"one match proves nothing"

Account D:
becomes a fan because clip went viral
```

## 6.26 Social media should contain hype and anti-hype simultaneously

If one performance causes huge praise, other accounts can immediately push back:

- "It's one match."
- "Calm down."
- "Do it for three months."
- "He did it against the 18th-place team."
- "You wanted him sold last week."

Stats and tactical accounts may react differently from emotional fan accounts.

## 6.27 Fame can outrun informed football reputation

A player may suddenly have:

```text
Fame: enormous
Football reputation: moderate
Commercial value: rapidly rising
```

Attention can later:

- convert into durable fame
- convert into real support
- fade rapidly
- collapse after poor performance
- survive through celebrity appeal

## 6.28 Viral waves have different half-lives

A meme may disappear in days.

A famous goal may remain culturally relevant for years.

A scandal may follow a player for a decade.

The system should distinguish:

- short attention spike
- medium-term trend
- durable cultural memory

Old material can reactivate when relevant.

## 6.29 Social media constructs mythology

Supporters should use current events to create jokes, myths, pseudo-prophecies, symbols, and folklore.

Example:

```text
random supporter jokingly posts:
"The prophecy said a left-footed number 17 would win us the league."

→ post goes viral
→ meme accounts repeat it
→ fan edits appear
→ chant references it
→ journalists mention nickname
→ newer fans misunderstand origin
→ older accounts explain the joke
```

The game does **not** need an actual prophecy.

The social world creates the myth.

## 6.30 Historical comparisons should emerge naturally

Fans should compare current players to:

- former stars
- club legends
- old academy products
- record holders
- previous title-winning squads
- notorious flops
- derby heroes
- national-team icons

Examples:

- "next [former player]"
- "best since [legend]"
- "already better than [record holder]"
- "stop comparing every academy kid to [legend]"

These comparisons can be accurate, exaggerated, nostalgic, ignorant, or joking.

## 6.31 Social actors have different historical knowledge

A younger casual fan may confidently misremember history.

An older lifelong supporter may correct them.

A stats account may know the records but not the cultural meaning.

A meme account may deliberately ignore accuracy for humour.

This diversity is desirable.

## 6.32 Records and real simulated history should feed social discourse

The social layer should be able to reference actual simulation history:

- career records
- club records
- competition records
- previous players
- old matches
- title seasons
- collapses
- derbies
- famous transfers
- shirt numbers
- chants
- controversies
- academy history
- national-team history

The system should not invent factual football-history claims that the simulation cannot support.

Jokes and myths may be invented by supporters, but factual record claims should still have a grounding path.

## 6.33 Historical knowledge can become generational conflict

Example:

```text
young supporter:
"Best striker we've had in 20 years."

older supporter:
"The disrespect to the 2009 team is insane."

stats account:
posts comparison

another supporter:
"Stats don't show what he meant to the club."
```

This is a desirable social pattern.

## 6.34 Nicknames can emerge, spread, mutate, and die

Nicknames should be able to originate from:

- goals
- appearance
- personality
- a viral clip
- commentator phrase
- fan joke
- geography
- shirt number
- weather/climate adaptation
- rivalry

A nickname may:

- catch on
- die quickly
- become a chant
- be adopted by journalists
- be used by sponsors
- be hated by the player
- be embraced by the player
- be twisted mockingly by rivals

## 6.35 Jokes can become genuine club culture

A throwaway joke can, through repetition, become part of supporter identity.

That history should persist.

Later users may not even know the original context.

This is a feature, not a bug.

## 6.36 Thirst comments are legitimate social behaviour

Some users will make attraction-driven comments that have little or nothing to do with football.

Examples of tones can include:

- playful admiration
- fan edits
- fashion/appearance comments
- explicit thirst-style social commentary within normal platform boundaries
- "I do not know football but I am following him now"

Those users are still real participants in the social ecosystem.

## 6.37 Negative appearance comments can also exist

Some people will make cruel or mocking comments about a player's looks.

Examples can include:

- mocking thumbnails
- saying they dislike seeing the player on screen
- insults about appearance
- jokes intended to humiliate

Other users may challenge those comments.

The simulation should understand these as **opinions expressed by individuals**, not objective narration.

## 6.38 Appearance hostility can be genuine or instrumental

A user may attack someone's appearance because:

- they genuinely find them unattractive
- they dislike the player for football reasons
- they support a rival
- they want engagement
- they are trolling
- they are joining a trend

The visible comment does not necessarily reveal the true motive.

## 6.39 Public conversation can contain multiple modes at once

The same viral thread may contain:

```text
tactical analysis
record comparison
nostalgia
one-match hype
anti-hype
thirst comments
appearance insults
memes
prophecy jokes
serious journalism
rival trolling
historical correction
commercial/fashion interest
```

That mixture is desirable.

## 6.40 Social culture should emerge from state, history, and audience

Conceptually:

```text
SOCIAL CULTURE
=
football opinion
+ identity
+ appearance
+ attraction
+ humour
+ history
+ records
+ nostalgia
+ mythology
+ memes
+ virality
+ tribalism
+ generational knowledge
+ brand value
+ platform culture
```

People can be:

- right
- wrong
- joking
- exaggerating
- nostalgic
- reactionary
- cruel
- clever
- uninformed
- contrarian
- dead serious

sometimes inside the same thread.

## 6.41 Non-negotiable separation of concepts

Do not collapse:

```text
ABILITY
REPUTATION
FAME
ATTENTION
ATTRACTIVENESS / AESTHETIC RESPONSE
COMMERCIAL VALUE
SOCIAL REACH
```

into one score.

A spike in attention can increase fame and commercial value.

It must not automatically increase football ability or informed football reputation.

## 6.42 Desired emergent examples

### Viral attractive-player wave

```text
training reel goes viral
→ non-football audiences discover player
→ thirst/fan-edit activity rises
→ follower growth spikes
→ sponsor interest rises
→ football accounts push back on superficial hype
→ player becomes culturally famous beyond football
```

### Fake prophecy becomes folklore

```text
fan makes prophecy joke
→ meme spreads
→ nickname develops
→ chant references it
→ journalists mention it
→ newer supporters think it is old club folklore
→ original post resurfaces years later
```

### One-match hype war

```text
player has huge game
→ reactionary accounts call him world-class
→ stats accounts caution against small sample
→ old critics get quote-posted
→ rivals mock the hype
→ next match becomes socially loaded
```

### Appearance backlash and counter-backlash

```text
player becomes widely discussed for appearance
→ positive attraction comments spread
→ hostile appearance comments appear
→ other supporters criticise the cruelty
→ social discussion becomes partly detached from football
```

### History argument

```text
supporter calls player best in decades
→ older fans invoke former legend
→ stats comparison goes viral
→ records are checked
→ nostalgia and generational conflict emerge
```

These outcomes should arise from persistent actors, real simulation history, platform dynamics, and changing public attention rather than from scripted story beats.

---

# 7. Tactical Intelligence, Match Adaptation, Player State, and Life-to-Football Effects

> **Status: LOCKED**

## 7.1 Core tactical pipeline

Managers must not read tactical engine truth directly.

The desired match-thinking loop is:

```text
PRE-MATCH BELIEF
↓
LIVE OBSERVATION
↓
PATTERN RECOGNITION
↓
TACTICAL HYPOTHESIS
↓
CONFIDENCE
↓
DECISION / ADAPTATION
↓
PLAYER EXECUTION
↓
OPPONENT RESPONSE
↓
NEW EVIDENCE
↓
POST-MATCH LEARNING
```

Every stage can be imperfect.

A manager can:

- prepare for the wrong shape
- fail to notice the important pattern
- notice the pattern but diagnose it incorrectly
- diagnose it correctly but choose a poor solution
- choose a good solution that players cannot execute
- make a poor change that succeeds through luck
- make a strong change that fails through execution or randomness

Outcome alone must not determine whether the reasoning was good.

## 7.2 Pre-match preparation uses imperfect information

Pre-match opponent knowledge can draw from:

- recent matches
- scouting
- analysts
- previous meetings
- opposition-manager tendencies
- likely line-up
- injuries
- suspensions
- tactical trends
- player roles
- set pieces
- home/away tendencies
- recent shape changes

The result is a belief dossier with uncertainty, not exact engine parameters.

Example:

```text
Expected shape: 4-3-3
Confidence: high

Likely pressing style: aggressive
Confidence: high

Likely left-wing movement: inside
Confidence: medium

Starting striker: uncertain
Confidence: low
```

The opponent can deliberately surprise the manager.

## 7.3 Managers observe football evidence, not hidden numbers

Managers should observe patterns such as:

- centre-backs repeatedly forced long
- defensive midfielder being screened
- right-back repeatedly defending 2v1
- opponent winger staying wider than expected
- repeated final-third entries without box penetration
- striker dropping into midfield
- opponent press weakening
- transition spaces appearing
- aerial mismatch
- particular passing lane repeatedly available

Do not expose:

```text
OpponentPressIntensity = 84
```

The system should generate football observations from match behaviour.

## 7.4 Observation and diagnosis are separate

This distinction is mandatory.

Example:

```text
Observed:
left-back keeps losing duels
chances keep arriving from that side
```

The true cause may be:

```text
opponent creates structural 2v1 overload
```

The manager may instead conclude:

```text
left-back is simply playing badly
```

and substitute the player.

The structural problem remains.

Human-like tactical mistakes should emerge from imperfect diagnosis.

Conceptually:

```text
OBSERVATION
→ one or more hypotheses
→ confidence in each hypothesis
→ chosen explanation
→ chosen response
```

## 7.5 Tactical intelligence is multidimensional

Do not reduce managerial tactical ability to one magical number.

Relevant capabilities can include:

- pre-match preparation
- opposition analysis
- pattern recognition
- match reading
- tactical knowledge
- adaptability
- system design
- game-state management
- substitution judgement
- set-piece understanding
- communication
- risk judgement

A manager can be an excellent system builder but mediocre live coach.

Another can be ordinary during preparation but brilliant at halftime.

Another can read games extremely well but be tactically stubborn.

## 7.6 Coaching licences are competence signals, not omniscience

Formal licences can affect:

- tactical education
- methodology
- vocabulary
- exposure to advanced concepts
- preparation quality

But:

```text
Pro Licence ≠ perfect tactical judgement
```

Experience, individual skill, philosophy, staff quality, and history still matter.

## 7.7 Philosophy affects the response

Managers can diagnose the same problem correctly and choose different solutions.

Against a high press, one manager may:

- add a dropping midfielder
- widen the centre-backs
- play through pressure

Another may:

- play directly
- attack second balls

Another may:

- keep the structure
- wait for the press to tire

Another may:

- draw the press deeper through the goalkeeper

There must not be a universal rock-paper-scissors tactical counter table.

## 7.8 Tactical solutions create trade-offs

Examples:

```text
Push fullbacks higher
+ width
+ overloads
+ attacking support
- transition security
- space behind
```

```text
Drop defensive line
+ protects depth
- concedes territory
- may isolate forwards
```

Managers alter structures. The match engine produces consequences.

## 7.9 Player capability constrains tactical ideas

A manager can understand the correct solution while lacking players capable of executing it.

Execution can depend on:

- technical ability
- physical ability
- role suitability
- tactical understanding
- role familiarity
- system familiarity
- fitness
- fatigue
- chemistry
- communication
- confidence
- current personal state

A clever manager does not automatically make any squad capable of elite positional football.

## 7.10 Tactical familiarity is system-specific

A player can understand "winger in a 4-3-3" while still being unfamiliar with:

- exact press triggers
- fullback rotation
- rest-defence responsibility
- half-space movement
- transition responsibility
- width timing
- receive-and-turn patterns

Tactical integration takes time.

This connects directly to the adaptation system.

## 7.11 Training creates the tactical menu

Managers should not invent an entirely alien system in minute 63 and have it executed perfectly.

Distinguish:

```text
well-drilled variation
```

from:

```text
emergency improvisation
```

Teams trained in several structures have more live flexibility.

Teams trained narrowly may execute emergency changes poorly.

## 7.12 Staff contribute tactical observations

Possible live input:

```text
Assistant:
"Their right-back keeps leaving space behind."

Analyst:
"They are progressing almost entirely through our left half-space."

Set-piece coach:
"They're leaving the far post."

Fitness staff:
"Our midfield intensity is falling."
```

The manager may trust, ignore, challenge, or partially use that information.

This depends on:

- staff competence
- role
- relationship
- previous accuracy
- manager trust
- confidence
- ego
- philosophy

## 7.13 Staff and managers can disagree

Disagreement is allowed to persist.

An assistant may be correct while ignored.

A manager may be correct while staff disagree.

Repeated successes or failures can alter internal credibility and future influence.

## 7.14 Opponents react back

Tactical adjustment cannot permanently "solve" the opponent.

Example:

```text
A attacks space behind B's fullback
↓
B notices
↓
B tells fullback to hold
↓
space disappears
↓
A must find a new route
```

High-level matches can produce:

```text
plan
→ counter
→ counter-counter
```

Managerial quality and personality determine how much adaptation occurs.

## 7.15 Managers have different thresholds for change

Some managers are:

- reactive
- patient
- stubborn
- nervous
- impulsive
- methodical

One may change after 15 bad minutes.

Another may require 40 minutes of evidence.

Another may never abandon the plan.

This is separate from raw tactical knowledge.

## 7.16 Game state matters beyond the current score

Managers can consider:

- score
- time
- aggregate score
- elimination context
- goal-difference needs
- league-table implications
- red cards
- fatigue
- fixture congestion
- future fixtures
- competition importance

A 0-0 league match is not the same as being 0-0 while 2-0 down on aggregate in minute 70.

## 7.17 Match importance changes acceptable risk

A manager may accept much more tactical exposure when:

- facing elimination
- needing a win to avoid relegation
- chasing a title
- needing goal difference

Risk appetite remains manager- and club-dependent.

## 7.18 Refereeing, bookings, and physical state can change tactics

Managers can react to:

- booked defenders
- dangerous individual matchups
- strict or permissive officiating
- player knocks
- reduced acceleration
- fatigue
- cramp
- reduced intensity

Possible actions:

- add cover
- reduce aggression
- substitute
- change matchup
- lower pressing demands
- accept the risk because alternatives are worse

## 7.19 Substitutions require intent

Substitutions can happen for:

- fatigue
- injury
- yellow-card risk
- poor performance
- tactical mismatch
- pace
- aerial ability
- creativity
- defensive security
- role change
- shootout preparation
- development
- time management

Important substitutions should retain an intent/reason.

Managers can make bad substitutions.

## 7.20 Halftime is a meaningful reasoning window

Halftime allows:

- more evidence
- staff discussion
- player feedback
- more complex explanation
- structural changes
- psychological intervention

Managers should be more capable of communicating complex changes at halftime than during open play.

## 7.21 Players can provide tactical feedback

Players may report:

- opponent movement
- marking difficulties
- space they are seeing
- pitch conditions
- pressing difficulty
- communication problems

Experienced captains and tactically intelligent players can be especially useful.

Managers may still ignore or misinterpret the input.

## 7.22 Communication quality matters

Recognising the right answer is not enough.

The manager must communicate it.

Execution can depend on:

- managerial communication
- shared language
- tactical vocabulary
- player intelligence
- relationship
- stress
- fatigue
- familiarity

## 7.23 Tactical memory persists across meetings

Managers and clubs can remember:

- previous opponent structures
- what caused problems
- what adjustments worked
- what failed
- recurring opposition tendencies

Opponents remember back.

This allows genuine tactical rivalries.

## 7.24 Organisational tactical memory exists

Analysts and staff can accumulate club-level knowledge.

Examples:

- manager often changes to 3-5-2 when trailing
- striker struggles when denied central space
- press regularly weakens after 70 minutes

Staff turnover can weaken, remove, or reinterpret that memory.

## 7.25 Tactical schools create priors, not scripts

Tactical schools can influence preferences for:

- possession
- pressing
- direct play
- transition
- low block
- width
- overloads
- positional play

But two managers from the same school can still behave very differently.

## 7.26 Tactical evolution is emergent

A successful style can spread:

```text
style succeeds
↓
others imitate
↓
league learns responses
↓
counter-strategies grow
↓
style evolves, branches, or declines
```

Personnel can also create new ideas.

A manager may invent a structure because of an unusual squad, succeed, and inspire imitation.

## 7.27 Managers develop over careers

Managers can learn from:

- assistants
- rivals
- failures
- new leagues
- new players
- tactical trends
- mentors
- analysis
- ageing squads

They can become:

- more adaptable
- more pragmatic
- more aggressive
- more conservative
- more possession-oriented
- more stubborn

Development is not guaranteed to be positive.

## 7.28 Tactical reputation must emerge from history

Claims such as:

> "Brilliant in-game manager"

or:

> "No Plan B"

should be supported by actual history:

- halftime changes
- comeback patterns
- repeated tactical success
- recurring inability to adapt
- opponent-specific adjustments

Do not assign such narratives arbitrarily.

## 7.29 Post-match analysis updates future beliefs

Staff and managers can analyse:

- what worked
- what failed
- what was misdiagnosed
- what the opponent changed
- which players struggled
- which assumptions were wrong

They can also learn the wrong lesson because football outcomes contain randomness.

A poor change followed by a lucky set-piece goal may be credited incorrectly.

## 7.30 Media interpretation can differ from internal tactical reality

After a match:

```text
Journalist:
"Manager was tactically outclassed."

Manager:
"Individual mistakes cost us."

Player:
"We weren't sure when to press."

Analyst:
"Initial plan was sound but execution collapsed."
```

These interpretations can coexist.

## 7.31 Human player experiences tactics locally

The footballer perspective should receive:

- role
- instructions
- role changes
- substitutions
- relevant tactical communication
- observable teammate/opponent behaviour

The user does not become omniscient merely because the engine has deep tactics.

Senior or trusted players may gain greater involvement in tactical conversation.

## 7.32 Players can resist, misunderstand, or fail to execute changes

Failure can result from:

- misunderstanding
- unfamiliar role
- low tactical intelligence
- language barrier
- poor communication
- fatigue
- low trust
- disagreement
- pressure

Do not assume deliberate disobedience.

## 7.33 Tactical risk can be knowingly accepted

Distinguish:

```text
manager failed to notice the danger
```

from:

```text
manager saw the danger
and accepted it because a goal was needed
```

This mirrors the locked transfer-risk principle.

## 7.34 Tactical decisions retain causal traces

Important changes should retain enough information to explain:

- why shape changed
- why player was substituted
- why pressing changed
- what problem the manager believed existed
- what outcome was expected

This supports:

- commentary
- media
- post-match reports
- manager reputation
- debugging

---

## 7.35 Every player carries a dynamic life state into football

A player is not only technical, physical, and tactical attributes.

Conceptually:

```text
PLAYER MATCH STATE
=
football ability
+ physical condition
+ tactical familiarity
+ confidence
+ concentration
+ emotional load
+ recent experiences
+ public pressure
+ relationships
+ personal life
+ sleep / recovery
+ motivation
+ perceived stakes
```

These factors do not all matter equally every day.

Many days will be ordinary.

At other times, a single life event can dominate a player's week.

## 7.36 Life events influence football through interpretation, not flat buffs

Do not implement:

```text
negative social media
→ -10 passing
```

Use:

```text
event
→ personal interpretation
→ psychological / behavioural state
→ football consequences
```

Example:

```text
online abuse
↓
rumination / anger / motivation / indifference
↓
sleep, confidence, focus, risk-taking, emotional control
↓
match behaviour
```

Different players can react completely differently to the same event.

## 7.37 Positive and negative experiences use the same machinery

Examples of relevant events include:

- family conflict
- relationship breakup
- new relationship
- parent illness
- new child
- marriage
- family relocation
- housing difficulty
- visa stress
- loneliness abroad
- financial pressure
- public controversy
- national-team call-up
- new contract
- supporter praise
- successful comeback
- career milestone

An event can create mixed effects.

Example:

```text
new child
→ happiness ↑
→ motivation ↑
→ sleep ↓
→ fatigue ↑
```

The system should support simultaneous positive and negative states.

## 7.38 State and trait are separate

Traits may include:

- professionalism
- resilience
- temperament
- social confidence
- emotional regulation
- risk preference
- coping style

Current state may include:

- confidence
- stress
- focus
- sleep quality
- anger
- grief
- excitement
- pressure
- motivation
- social load
- belonging

Traits influence how state develops.

State influences present behaviour.

History influences both.

## 7.39 Effects have temporal profiles

Meaningful states can have:

- onset
- peak intensity
- expected duration
- actual duration
- decay
- triggers
- recovery factors
- possible recurrence

Some effects may meaningfully influence:

- one match
- two matches
- one week
- two weeks
- one month
- several months

Major experiences can leave a long-term memory after their acute effect ends.

Do not reduce all events to a single fixed timer.

## 7.40 Trauma and major memories can reactivate

Example:

```text
serious injury in a final
↓
recovery
↓
one year later returns to same stadium
↓
memory becomes salient
```

Possible reactions can include:

- nervousness
- heightened focus
- avoidance
- anger
- motivation
- little meaningful reaction

Likewise, a missed decisive penalty can become relevant when the player later faces another high-pressure penalty.

Psychology changes probabilities and behaviour, not destiny.

## 7.41 Extreme pressure does not guarantee poor performance

A player can be:

```text
Confidence: high
Stress: extreme
Motivation: very high
Sleep: poor
Public pressure: enormous
Family support: strong
```

and still perform brilliantly.

Likewise a settled player can play badly.

The game must support stories such as:

> "He was under extraordinary pressure and still scored six goals in four matches."

The world can then react to that achievement.

## 7.42 The world notices performance under context

If a player performs despite visible stress or adversity, reactions can include:

- supporter admiration
- teammate respect
- manager praise
- media resilience narratives
- sponsor interest
- family pride
- increased public identification

If performance drops, reactions may diverge:

- sympathy
- criticism
- manager blame
- arguments that the player should have been rested
- claims that personal issues are irrelevant

The social system interprets the football result through known context.

## 7.43 Player support networks matter

Relevant people can include:

- family
- partner
- friends
- teammates
- captain
- manager
- assistant
- agent
- mentor
- club staff
- psychologist / wellbeing staff where applicable

Support quality changes outcomes.

A strong support network can help recovery.

Isolation, conflict, exploitation, or poor handling can worsen the situation.

Support attempts can themselves be misinterpreted and make things worse.

## 7.44 Managers perceive personal state imperfectly

A manager may know a player is struggling.

Another may notice only poor training.

Another may know nothing because the player hides it.

Possible responses include:

- start
- bench
- rest
- send home
- simplify role
- reduce minutes
- protect publicly
- say nothing publicly
- seek support staff input

Manager knowledge is still belief-based.

## 7.45 Player state can influence tactical choices

A technically available player may still be:

- poorly rested
- emotionally overloaded
- distracted
- lacking confidence
- physically drained

The manager may decide that another player is better **today**, even if the first is stronger in abstract ability.

Conversely, a confident player in excellent form may receive more freedom.

## 7.46 Match events feed back into life state

The loop continues during the match.

Example:

```text
player already under pressure
↓
early mistake
↓
crowd groans
↓
confidence falls
↓
plays safer
```

or:

```text
early mistake
↓
captain encourages player
↓
wins next duel
↓
crowd responds positively
↓
player settles
```

State is dynamic, not a pre-match modifier frozen until full time.

## 7.47 UI exposes contextual player state for every player, subject to perspective

This system exists for all meaningful players, not only the human-controlled footballer.

The UI should show relevant known context.

For the human player, more detail may be legitimately known:

```text
Current state:
High pressure

Confidence:
Stable

Sleep:
Poor recently

Emotional load:
Elevated

Known contributors:
- family issue
- intense online attention
- contract uncertainty
```

For a teammate:

```text
Seems distracted recently.
Manager mentioned personal matters.
```

For an opponent:

```text
Returned after compassionate leave.
```

Or nothing, if the information is private.

Never expose hidden psychological truth merely because the engine knows it.

## 7.48 UI should communicate time horizon without gamey certainty

The interface may communicate:

- immediate
- very short term
- short term
- several weeks
- ongoing

or contextual wording such as:

> "Likely to remain under heavy pressure in the short term."

Avoid:

```text
Stress debuff expires in 9.4 days
```

unless a concrete external circumstance genuinely provides that certainty.

## 7.49 Social discourse can cross into training, life, and matches

A viral pile-on can affect:

```text
social
→ player state
→ teammates
→ manager
→ press questions
→ family
→ training
→ match
```

A strong performance can then feed back:

```text
match
→ social praise
→ public narrative
→ confidence
→ relationships
```

This is one continuous causal loop.

## 7.50 Public hype can create pressure or energy

A 17-year-old can go:

```text
20k followers
→ viral performance
→ 1.8m followers
→ "future of the club"
→ legend comparisons
→ prophecy memes
```

Possible player reactions:

- loves attention
- becomes overwhelmed
- becomes arrogant
- becomes motivated
- fears disappointing everyone
- stays grounded because support network is excellent

Fame itself is not the effect. Interpretation is.

## 7.51 Wrongdoing and personal incidents propagate contextually

If a player humiliates a teammate, potential impact can reach:

- victim
- victim's friends
- captain
- manager
- dressing room
- supporters
- journalists
- sponsors
- board
- family
- agent

But not everyone must react.

Magnitude depends on:

- severity
- publicity
- evidence
- relationships
- hierarchy
- culture
- victim popularity
- offender popularity
- previous behaviour
- apology
- perceived sincerity
- media amplification
- who witnessed it

Never implement:

```text
scandal
→ everyone -10 opinion
```

## 7.52 Some events remain private until later

An incident may initially be known only by:

- direct participants
- witnesses
- a trusted confidant

Later it can leak.

Then:

```text
OLD EVENT
+
NEW INFORMATION
=
NEW CONSEQUENCES
```

The event can be old while its public life is new.

## 7.53 Performance and morality do not erase each other

A player can behave badly and continue scoring.

Different people can respond:

- "I don't care, he's winning."
- "Being good does not excuse it."
- "I dislike what he did but he has been incredible."
- "The club should have dropped him regardless."

This is exactly why opinion must remain multidimensional.

## 7.54 Managers face human decisions, not only selection maths

A star under extreme personal stress creates a real decision involving:

- football importance
- wellbeing
- player request
- alternatives
- staff advice
- public pressure
- team response
- match importance

Whatever the manager chooses becomes another event the world may judge.

---

# 8. Perspective-Safe UI and Hidden-State Information Firewall

> **Status: LOCKED**

## 8.1 The simulation owns truth; the view layer owns visibility

The fundamental boundary is:

```text
WORLD STATE
↓
PERSPECTIVE / KNOWLEDGE RESOLUTION
↓
VIEW MODEL
↓
UI
```

The normal player-facing UI must never receive raw hidden truth simply because it exists inside the simulation.

## 8.2 Every request has a perspective

Examples:

- human-controlled footballer
- manager
- sporting director
- journalist
- public
- debug/developer omniscient mode

The same entity can expose different information to different perspectives.

## 8.3 Exact values require a legitimate reason

Exact known facts can include:

- wage
- contract expiry
- appearances
- goals
- official transfer fee
- official match statistics

Private or uncertain concepts should normally be represented as:

- labels
- ranges
- confidence
- observations
- evidence
- inferred beliefs

Examples:

```text
Manager trust:
Strong
```

rather than:

```text
Manager trust:
742 / 1000
```

## 8.4 Information has epistemic status

Useful conceptual states include:

```text
PUBLIC FACT
PRIVATE FACT
PERCEIVED FACT
INFERRED BELIEF
REPORT / RUMOUR
UNKNOWN
```

Example:

```text
Contract expires June 2030
→ public fact

Club privately wants to sell player
→ private fact

Player believes manager trusts him
→ perceived fact

Agent suspects another club has interest
→ inferred belief

Journalist reports interest
→ report / rumour

Internal buyer-interest score
→ unknown to player
```

## 8.5 Confidence can be communicated semantically

Examples:

- almost certain
- strongly suspects
- believes
- has heard
- press speculation
- no reliable information

Do not convert uncertain knowledge into false exact percentages merely for convenience.

## 8.6 Sorting and filtering can leak hidden truth

A screen can leak information even when the raw value is hidden.

Examples:

```text
"Most talented prospects"
sorted by true PA
```

or:

```text
"Most likely transfer destinations"
sorted by hidden internal interest
```

Every ranking must answer:

```text
Sorted according to whose belief?
```

Filters and search must be perspective-safe too.

## 8.7 Relationship screens show perception and evidence, not soul meters

Prefer:

```text
Relationship:
Close

Trust:
Strong

Current tone:
Warm

Recent evidence:
- defended you publicly
- spent time together
- tension from last month has eased
```

over:

```text
affinity = 812
trust = 631
respect = 744
```

A person can secretly resent the human player while appearing friendly.

The UI must not spoil that by exposing the internal relationship struct.

## 8.8 Debug omniscience is explicitly separate

Development tools may expose exact truth.

They must be clearly labelled and isolated from normal UI code paths.

## 8.9 The information firewall should be enforced in architecture

Do not rely only on developer discipline.

Normal pages should consume perspective-filtered view models or accessors rather than raw `World` internals.

Conceptually:

```text
VisiblePlayer
VisibleClub
VisibleRelationship
VisibleTransferInterest
```

rather than internal simulation structures.

## 8.10 Hidden-state leak tests are required

Representative invariants:

- human-player view does not expose true PA
- private manager trust is not exposed exactly
- public pages cannot read private injury diagnosis
- unreported buyer interest is not exposed
- relationship internals are not exposed
- prospect lists do not sort by true PA
- public search cannot filter by hidden personality

## 8.11 Hidden information can become visible through legitimate channels

Information can emerge through:

- manager conversation
- agent
- journalist
- leak
- teammate
- public document
- repeated observation

Then the view changes because the actor's knowledge changed.

---

# 9. Strongly Typed Frontend / Backend API Contracts

> **Status: LOCKED**

## 9.1 Contract drift should fail early

If the backend request or response changes, the frontend should fail at:

- compile time
- binding generation
- schema validation
- CI contract tests

not only after a user clicks a screen.

## 9.2 Avoid production `any` and stringly-typed payload drift

The target is closer to:

```text
api.playerDetails(PlayerDetailsRequest)
→ PlayerDetailsView
```

than:

```text
call<any>("player_details", arbitrary_json)
```

## 9.3 One authoritative contract source

Avoid manually maintaining three separate truths:

- Rust structs
- TypeScript interfaces
- prose docs

Prefer Rust-defined or shared-schema-defined contracts with generated/validated TypeScript bindings.

## 9.4 API payloads must be view types, not world types

The API boundary reinforces Section 8.

The frontend should receive perspective-safe structures, not internal structures containing hidden fields that developers merely promise not to render.

## 9.5 Errors should be structured

Useful categories can include:

- NotFound
- UnauthorizedPerspective
- InvalidRequest
- StateConflict
- UnavailableInformation
- SaveIncompatible
- SimulationBusy
- InternalError

The frontend should be able to distinguish "you do not know this" from "the program failed."

## 9.6 Unknown, hidden, estimated, and known are semantically distinct

Do not use `0`, `false`, or `null` carelessly where they can collapse meaning.

Where appropriate, prefer explicit semantic variants such as:

```text
Known(value)
Estimated(range, confidence)
Reported(value, source)
Unknown
Hidden
```

## 9.7 Separate queries from commands

Queries ask for visible state.

Commands express player intent.

Example:

```text
Query:
get contract offer

Command:
accept offer { offer_id }
```

Do not let the frontend submit authoritative world state such as a new wage amount when the action is merely "accept the existing offer."

## 9.8 Backend owns reality; frontend owns presentation and intent

Conceptually:

```text
BACKEND
owns truth, rules, validation, world mutation

FRONTEND
owns presentation and player intent
```

## 9.9 Contract tests should cover major surfaces

Representative areas:

- player profile
- club overview
- social feed
- transfer interest
- contract offer
- relationship view
- news story
- save metadata

---

# 10. Save Evolution, Deterministic Migrations, and Historical Continuity

> **Status: LOCKED**

## 10.1 Save schema version is not game version

Schema versions change only when persistent data representation changes incompatibly.

Use a clear sequence such as:

```text
2 → 3 → 4 → 5
```

not a save schema for every application release.

## 10.2 Migrations are sequential

A Schema 2 save loaded by a Schema 5 build should migrate:

```text
2 → 3
3 → 4
4 → 5
```

Small explicit migrations are preferred over giant pairwise converters.

## 10.3 Migration cannot invent historical certainty

If an old save stored:

```text
opinion = +63
```

and the new system has many dimensions, migration cannot pretend the old save contained all of them.

It can:

- derive what is defensible
- initialise neutral/unknown values
- mark legacy provenance

Do not fabricate history that never existed.

## 10.4 Unknown can be better than false zero

If an old schema never stored a concept, distinguish where relevant:

```text
we know this was neutral
```

from:

```text
this data did not exist yet
```

## 10.5 Derived caches should normally be rebuilt

If a value can safely be regenerated from authoritative state, prefer rebuilding it after migration rather than migrating stale derived data.

## 10.6 Persistent entity IDs are sacred

Migrations should preserve identity for:

- people
- clubs
- contracts
- journalists
- social accounts
- relationships
- stories
- transfers

Regenerating IDs can destroy the causal graph of the world.

## 10.7 Causal memory survives migration

Preserve not only current state but important reasons and history.

A five-year journalist/player feud should not migrate into a naked hostility number with all causes deleted.

## 10.8 New systems receive deterministic legacy initialisation

If a new subsystem did not exist before, initialise it from:

- existing world state
- documented neutral baseline
- deterministic derived state

Never use uncontrolled randomness.

If synthetic generation is necessary:

```text
stable_seed(
    world_seed,
    migration_id,
    entity_id
)
```

Same save + same migration must always yield the same result.

## 10.9 Backup and atomicity are mandatory

Desired flow:

```text
read old save
↓
verify
↓
preserve untouched backup
↓
migrate to temporary state
↓
validate
↓
write atomically
```

Failure must leave the original intact.

## 10.10 Validate migration invariants

Examples:

- all referenced IDs exist
- contracts reference valid parties
- rosters reference valid players
- relationship endpoints exist
- no duplicate persistent IDs
- competition memberships remain valid
- dates are sane
- finances are sane

Run stronger validation after the full chain.

## 10.11 Keep representative old-save fixtures

CI should load real old-schema fixtures and test:

```text
old save
→ migrate
→ validate
→ simulate
→ save
→ reload
→ simulate
```

Keep ugly edge cases too:

- active transfer
- active contract negotiation
- injured player
- loan
- active media saga
- relationship grievance
- manager recently fired
- competition mid-season
- social thread in progress

## 10.12 Compatibility window can be finite

It is not necessary to promise infinite support for every ancient development build.

But once public long-term careers matter, migration support should be generous and clearly defined.

## 10.13 Downgrade migrations are generally unsupported

A save created by a newer schema should not be silently downgraded into an older build.

## 10.14 Data reconciliation and schema migration are different problems

Distinguish:

```text
persistent save structure changed
```

from:

```text
external database / imported source data changed
```

These require different tools and diagnostics.

## 10.15 Save metadata should aid debugging

Useful metadata can include:

- created schema
- current schema
- game build
- world seed
- migration history
- import provenance version

## 10.16 Migrations preserve the universe, not rewrite history

If an old version produced strange but valid contracts, transfers, results, or opinions, migration should not rewrite the past merely because newer simulation logic is smarter.

Past legitimate outcomes belong to that universe.

---

# 11. Imported Data, Inference, Calibration, Provenance, and Non-Circular Valuation

> **Status: LOCKED**

## 11.1 Imported data is evidence used to initialise a world

The import pipeline should distinguish:

```text
Imported
Inferred
Generated
Unknown
```

These statuses must remain meaningful.

## 11.2 Avoid circular inference

Dangerous loop:

```text
market value
→ infer ability
→ ability drives performance
→ performance drives reputation/value
→ resulting value appears to validate ability
```

Initialisation must avoid turning one noisy proxy into a self-confirming truth machine.

## 11.3 Market value may be evidence but is never the ability oracle

Useful inference can combine:

- league strength
- club level
- minutes
- starts
- age
- position
- competition level
- international appearances
- historical transfers
- market value
- wage
- statistics
- awards
- squad role
- career trajectory

No single field should dominate universally.

## 11.4 Evidence has reliability and context

A source can carry:

- origin
- confidence
- date
- scope

Official appearances may be high-confidence.

Third-party valuation may be medium-confidence.

An unreliable inferred wage may be weaker.

## 11.5 Market context changes the meaning of money

The same nominal fee/value can mean very different things across:

- Premier League
- India
- Brazil
- Croatia
- Belgium
- Saudi Arabia
- Argentina
- other football economies

Value-to-ability inference must be context-calibrated.

## 11.6 Age changes valuation meaning

A valuable teenager's price may contain a large future-expectation component.

An older player's price may reflect current usefulness much more strongly.

Do not feed the entire valuation directly into current ability.

## 11.7 Contract leverage distorts price

A stronger player with six months remaining can cost less than a weaker player with five years remaining.

Fees can also be distorted by:

- release clauses
- forced sale
- relegation
- player pressure
- club cash need
- bidding war

Transfer fee is evidence of what somebody paid, not perfect evidence of football ability.

## 11.8 Reputation, fame, ability, and potential remain separate

A famous declining veteran can have:

```text
high reputation
lower current ability
```

An unknown youngster can have:

```text
high latent ability
low reputation
```

Do not move these concepts in lockstep.

## 11.9 Potential inference requires greater caution

Young + valuable does not automatically mean world-class ceiling.

The market can be wrong.

Potential initialisation should remain uncertain and evidence-informed.

## 11.10 Missing remains missing where inference would create fake precision

Do not infer:

- professionalism
- leadership
- temperament
- deep personality

from transfer value unless legitimate evidence exists.

Generated values should be marked generated.

## 11.11 Generated attributes should preserve plausible correlations

Missing data should not become independent random noise.

Generation can condition on:

- position
- age
- height
- career history
- league level
- known statistics

while preserving variation and outliers.

## 11.12 Calibration is population-level

Audit distributions such as:

- ability by league
- ability by age
- ability by position
- value vs ability
- value vs age
- wage vs club level
- minutes vs quality
- international selection vs league level
- development distributions
- retirement age
- injury rates
- fee distributions

Catch broad absurdities instead of tuning only famous players.

## 11.13 Calibration uses cohorts, not celebrity hand-tuning

Useful cohorts:

- elite top-league starters
- mid-table starters
- lower-league regulars
- reserves
- academy players
- veterans
- senior internationals
- youth internationals

Individual stars can be sanity checks, not the calibration objective.

## 11.14 Avoid double-counting correlated evidence

Market value may already encode:

- age
- league
- output
- reputation

An inference system using all of these independently can overweight the same underlying signal.

Calibration must account for correlated evidence.

## 11.15 Importer truth and in-world belief are different epistemic layers

Import inference can initialise engine truth.

It does **not** give clubs perfect knowledge.

Example:

```text
Importer confidence in latent potential: high
```

does not mean:

```text
Manager knows the player's potential accurately
```

The knowledge system still applies.

## 11.16 Source disagreement should preserve provenance

When sources disagree about:

- date of birth
- position
- nationality
- club
- fee
- transfer date
- height
- contract expiry

the resolution path should be traceable.

## 11.17 Derived initial values are simulation decisions, not source facts

If the importer infers ability, the provenance must say so.

Do not later claim the source provided that value.

## 11.18 Once simulation begins, the imported database stops controlling the future

After Day 0, the Rise Above universe owns:

- development
- injuries
- performances
- reputation
- transfers
- values
- finances
- media
- social history

External source data is the launchpad, not the puppeteer.

## 11.19 Import invariants are required

Examples:

- valid IDs
- valid ages
- valid club membership
- no NaN/impossible values
- valid contract dates
- missing remains semantically missing
- generated values have provenance
- inferred values have provenance
- same input + same seed produces same world

## 11.20 Outliers remain possible

Calibration should make unusual profiles rare, not impossible.

Examples:

- late bloomer
- tiny centre-back succeeding
- expensive flop
- cheap elite signing
- highly technical but slow player
- teenager already at elite level

Population realism must not flatten individuality.

## 11.21 Transfer-price modelling follows the locked negotiation model

This section must be read together with Sections 3.25-3.30.

Market value has less authority over an actual offer than:

- seller willingness
- player willingness
- contract leverage
- competition
- information
- local economics
- urgency
- scouting quality

A foreign club can legitimately buy a player from a low-wage or weakly exposed market for a relatively tiny amount if seller and player accept.

Later information about rival scouting or an upcoming bid can radically change negotiation behaviour without changing the player's underlying ability.

---

# 12. Documentation Hierarchy, Implementation Status, and Source-of-Truth Discipline

> **Status: LOCKED**

## 12.1 Separate intent from implemented reality

Use this hierarchy:

```text
LOCKED DESIGN
→ what the game is intended to mean

CODE + TESTS
→ what is actually implemented now

IMPLEMENTATION STATUS
→ current gap between design and code

ADRs
→ why important architectural decisions exist

HISTORICAL REPORTS
→ what was true at an earlier branch/commit
```

## 12.2 Code does not become wrong because a stale document says otherwise

If code and tests clearly implement something that an old status report calls missing, update the status report.

Do not rebuild working systems merely to match stale prose.

## 12.3 Design docs do not claim implementation

This document can say:

> Clubs reason under uncertainty.

That does not mean the current repository already implements every required part.

Implementation status must be tracked separately.

## 12.4 Implementation status should use explicit labels

Useful statuses:

- IMPLEMENTED
- PARTIAL
- SCAFFOLDED
- NOT IMPLEMENTED
- DEPRECATED
- NEEDS AUDIT

Testing can be a separate dimension.

Avoid vague phrases such as:

> "mostly complete"

where precision is practical.

## 12.5 Historical audits remain historical

Old reports should identify:

- repository
- branch
- audited commit
- date
- warning that implementation status may now be stale

Historical analysis can remain useful without pretending to describe current reality.

## 12.6 Every meaningful audit should be branch/commit-aware

This is especially important in a multi-branch integration workflow.

A statement that was correct on an older backend branch may be false on the current merged integration branch.

## 12.7 Coding agents verify code before acting on documentation

A future Codex instruction should require:

```text
Read design documents for intent.
Inspect current branch for reality.
Never implement a "missing" feature solely because an old report says it is missing.
```

Extend existing architecture where possible instead of building duplicate systems beside it.

## 12.8 Important architectural choices deserve lightweight ADRs

Examples:

```text
ADR:
Player-facing UI cannot access raw World.

Reason:
Perspective-safe hidden-state simulation.
```

```text
ADR:
Rust/shared schema owns API contracts.

Reason:
Prevent TypeScript/Rust drift.
```

ADRs should preserve the reasoning behind choices that future developers may otherwise "simplify" away.

## 12.9 Lock semantics, not arbitrary implementation details

A design rule such as:

```text
clubs decide under uncertainty
```

is durable.

A rule such as:

```text
must use exactly 17 f32 fields in ClubRiskModel
```

should not be locked unless genuinely necessary.

Implementation is allowed to improve while preserving semantics.

## 12.10 Generate mechanical documentation where practical

Machine-readable facts such as:

- workspace crates
- API commands
- save schema version
- registered migrations
- test inventory

should be generated or automatically checked where practical to reduce drift.

## 12.11 A canonical implementation-status document is desirable

One concise current-state file can answer:

> What exists right now?

It should not become another design bible.

## 12.12 Audit findings should have a lifecycle

Conceptually:

```text
AUDIT FINDING
↓
DESIGN DECISION
↓
IMPLEMENTATION
↓
TEST
↓
STATUS UPDATE
↓
CLOSED
```

This prevents repeated rediscovery of already-fixed issues.

## 12.13 Documentation never overrules runtime evidence

If determinism tests fail, determinism is broken even if a document says otherwise.

If the design prohibits true-PA leaks but the frontend payload contains true PA, implementation violates the design.

Documentation defines intent; tests and code reveal reality.

---

# 13. Unified Causal Simulation Architecture

> **Status: LOCKED — FOUNDATIONAL**

## 13.1 Rise Above is one world, not many disconnected systems

The game must not behave as:

```text
football module
social module
life module
media module
relationship module
transfer module
```

that occasionally exchange scripted bonuses.

The target is:

```text
ONE PERSISTENT WORLD STATE
```

in which football, life, relationships, media, contracts, family, club politics, psychology, public opinion, reputation, and economics are different interacting views of the same causal world.

## 13.2 No subsystem owns consequences exclusively

A meaningful event can create consequences anywhere there is a legitimate causal path.

Example:

```text
breakup
↓
poor sleep
↓
training dip
↓
manager notices
↓
benching
↓
press speculation
↓
online discourse
↓
player reacts publicly
↓
supporters divide
↓
sponsor becomes nervous
↓
teammates intervene
↓
player settles
↓
form returns
↓
comeback narrative
```

No "breakup storyline" needs to script that entire chain.

The systems create it together.

## 13.3 Core causal pattern

A broad cross-system loop is:

```text
EVENT
↓
WHO KNOWS?
↓
HOW DOES EACH PERSON INTERPRET IT?
↓
STATE CHANGE
↓
DECISION / BEHAVIOUR
↓
WHO OBSERVES THAT?
↓
RELATIONSHIP / PUBLIC / ORGANISATIONAL CONSEQUENCES
↓
NEW EVENTS
```

This applies to:

- tactical events
- family events
- social events
- wrongdoing
- transfer negotiations
- contract disputes
- media stories
- injuries
- performances
- supporter reactions

## 13.4 Consequence magnitude is contextual

One wrongdoing can affect:

- only two people
- a small friendship group
- the dressing room
- the club
- a national audience
- the entire football world

depending on:

- severity
- visibility
- credibility
- relationships
- culture
- status
- fame
- hierarchy
- prior history
- platform reach
- media amplification
- timing

Do not force every event to have universal impact.

## 13.5 Knowledge propagation is part of the simulation

An event can be:

- private
- witnessed
- privately shared
- rumoured
- leaked
- publicly confirmed

Different people can know different versions of it.

Consequences therefore depend not only on what happened but on **who believes what happened**.

## 13.6 Positive and negative outcomes share one framework

Do not create a sophisticated negative-event system and a separate "+morale" positive-event system.

The same architecture handles:

- joy
- belonging
- love
- praise
- success
- grief
- conflict
- pressure
- embarrassment
- scandal

Mixed states are normal.

## 13.7 Time is part of causality

Effects can last:

- one moment
- one match
- two matches
- several days
- several weeks
- months
- years as memory

Duration emerges from the event, person, coping, environment, support, and later developments.

## 13.8 Systems can amplify or damp one another

Example:

```text
bad match
↓
online criticism

Strong support network:
criticism damped
↓
player stabilises

Weak support network:
criticism amplifies stress
↓
training worsens
↓
another bad match
↓
criticism grows
```

The second path creates a feedback loop.

The first breaks it.

## 13.9 Feedback loops must be possible but not automatically runaway

The simulation should permit:

- confidence spirals
- reputation snowballs
- media pile-ons
- recovery arcs
- transfer auctions
- tactical learning loops
- supporter mythology

but should include realistic damping through:

- changing evidence
- time
- support
- competing narratives
- fatigue of attention
- institutional intervention
- new performances
- random variation

## 13.10 Causal memory is essential

Important state changes should remember why they happened.

A value without history is often insufficient.

Examples:

```text
trust low
because player lied about transfer intentions

media hostility high
because private information was published

supporter affection high
because player performed during family crisis

manager confidence low
because assistant repeatedly identified issues first
```

Future systems can react differently depending on the cause.

## 13.11 Reactions can themselves become events

A response is not merely an output.

Example:

```text
player criticised online
↓
player posts angry reply
↓
reply becomes viral event
↓
club reacts
↓
manager relationship changes
↓
journalists ask questions
```

The world continues from the reaction.

## 13.12 The human player is not the centre of causality

All meaningful players, managers, staff, journalists, clubs, agents, supporters, and relationships can participate in the same systems.

The protagonist is not granted special physics.

The world continues even when events do not involve the user.

## 13.13 Information, belief, and consequence remain separate

The world can contain:

```text
what actually happened
what a person saw
what they believe happened
what they tell others
what others believe
what consequences follow
```

These layers must not collapse into one truth variable.

## 13.14 The design goal is compositional depth

The desired complexity is not "10,000 isolated modifiers."

It is a smaller set of coherent state variables and causal rules that combine into enormous numbers of situations.

The richness should come from composition:

```text
personality
+ history
+ relationships
+ knowledge
+ pressure
+ culture
+ football context
+ institutional incentives
+ timing
```

rather than thousands of bespoke scripted cases.

## 13.15 Final foundational rule

```text
Every system should be able to influence another system
when a believable causal pathway exists.

No system should influence another merely because
the game wants a dramatic outcome.

State creates pressure.
Pressure changes probabilities and decisions.
People interpret events.
Those decisions create consequences.
Consequences become new state.
```

This is the central simulation philosophy for Rise Above.

---

# Final Locked Design Summary

Rise Above is a persistent football-and-life simulation in which:

- actors operate on imperfect knowledge rather than hidden engine truth
- managers, scouts, players, agents, journalists, supporters, and clubs can be wrong
- media separates truth, sourcing, framing, intent, perception, and outcome
- relationships are directional and remember why they changed
- transfer prices emerge from negotiation, leverage, scouting asymmetry, information, and context rather than public market value
- contracts vary structurally by club, player, leverage, risk, strategy, and legal context
- squad planning uses uncertain scenarios and time-to-usefulness rather than exact future ability
- adaptation spans climate, routine, football, culture, language, social life, and psychology
- social opinion is multidimensional and can include football judgement, affection, trust, attraction, cruelty, brand interest, nostalgia, history, memes, mythology, and virality
- supporters can create fake prophecies, nicknames, chants, historical comparisons, and folklore that become real culture through repetition
- tactical intelligence uses observation, diagnosis, staff input, philosophy, risk, execution, opponent response, and memory
- player life state travels onto the pitch and can influence one match, several matches, weeks, months, or long-term memory
- positive and negative personal events use one unified state system
- player pressure and adversity can hurt performance, help performance, do both, or have little effect depending on the person and context
- the UI exposes only perspective-valid knowledge
- frontend/backend contracts preserve the distinction between known, estimated, reported, hidden, and unknown information
- saves evolve through explicit deterministic migrations without rewriting legitimate world history
- imports preserve provenance and do not create self-confirming valuation loops
- documentation separates intended design, implemented reality, implementation status, architectural rationale, and historical snapshots
- every meaningful subsystem participates in one causal world rather than operating as a disconnected feature island

The world should produce stories because people, institutions, information, relationships, football, economics, and life continuously affect one another.

The design target is not scripted drama.

It is **coherent causality that naturally creates drama, mundanity, mistakes, brilliance, luck, unfairness, recovery, obsession, culture, and history**.

