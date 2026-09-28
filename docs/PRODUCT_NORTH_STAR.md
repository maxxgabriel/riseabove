# Product North Star

This is the reference for what Rise Above (Pathway) is. When this document conflicts with `plan/`,
`foundation/` or older notes, this document wins. Companion documents:
[`SYSTEMIC_SIMULATION_RULES.md`](SYSTEMIC_SIMULATION_RULES.md) (how we build it) and
[`ANTI_LINEAR_DESIGN_CHECKLIST.md`](ANTI_LINEAR_DESIGN_CHECKLIST.md) (how we catch drift).

---

## 1. The game in one paragraph

Rise Above is a general, autonomous football-and-life simulation. A whole football world (nations,
leagues, clubs, boards, managers, staff, scouts, agents, journalists, players and their families) lives
day by day for as long as the program runs, with or without anyone watching. The person using the program
can step inside that world and **inhabit one person**: take over their decisions, see what they see, and
live with the consequences. That person gets no special rules. The only difference between them and
everyone else is where their decisions come from: a human instead of the AI. When the human steps out,
the AI takes that person's decisions back, and the world carries on as it always would have.

It is Football Manager's world with the manager's chair removed. You are one of the people in the
simulation, not the person running the simulation.

## 2. What the experience is

- **Living inside a machine, not following a path.** Nothing is waiting for you. A club signs you
  because its squad plan has a hole, its scout saw you, its board has money and its manager rates you.
  It is not because you reached a point in a storyline. The same machinery moves every other player in
  the world.
- **Consequences with memory.** A manager remembers the argument in March, the transfer request in
  August and the extra sessions you put in over the winter. Fans remember that you joined their rivals. Your
  knee remembers that you rushed back. These memories change how people act later.
- **Uncertainty.** You never see the truth directly. You see your coach's opinion of your ability, your
  agent's account of the interest in you, a journalist's version of a rumour, and your own tired legs. Part of
  the game is working out what is real.
- **Width and depth from interaction.** Hundreds of small systems each weigh many factors and feed each
  other. The complexity is combinatorial, not handwritten.
- **Football first, life close behind** (about 60:40). Both sides are deep. Family, partner, friends,
  money, home, language, health and education are made of people and state, not flavour popups.
- **Text and data.** There are no graphics to build. The whole budget goes into logic, state, history,
  relationships, causality, and text that describes that state well.
- **No ending.** Retirement, injury, relegation and being released are transitions. The person
  continues as a coach, scout, agent, pundit or private citizen, or just as a name in the record books,
  and the world never stops. The user can stop playing a person. The universe does not end.

## 3. What it is not

Not a story game, a chapter-based career, a finite campaign, a sequence of scripted events, an
"A/B/C then continue" game, a career mode with a secretly privileged hero, a game with a designed
ending, a game where events fire because the protagonist reached stage X, or a set of 20 or 100
handcrafted career moments. It is not a game where the UI is the game.

## 4. Who you can be

Any eligible person at any point in their life: a 12-year-old in an academy, a 17-year-old scholar, a
21-year-old semi-pro, a 27-year-old international, a 34-year-old veteran, a generated regen, an imported
real-data player. The mode can later allow switching to another person. Taking control means
`AiMind → HumanMind` on an existing person, and giving it up is the reverse. Nothing is recreated.

"New career" in the UI is therefore one of two things:
1. Pick an existing person in a running world.
2. Ask the world to generate a new person through the same generator it uses for everyone (youth
   intake, regens, families), then pick them.

## 5. The three truths the simulation must hold

1. **The world is primary.** It must be able to run for decades with nobody controlled and still
   produce believable football history: dynasties, collapses, careers, records, managers rising and
   falling.
2. **The controlled person is ordinary.** Every opportunity they get, an AI player in the same
   circumstances could also get. Every setback they suffer, an AI player could also suffer.
3. **Text describes state.** Inbox messages, news, social media, conversation lines and reports are
   renderings of things that actually happened or are actually true (or actually believed by the
   speaker). Text never creates facts on its own.

## 6. What a good moment looks like

A manager calls you into his office on a Thursday. Nothing scheduled this. He has high Discipline, your
training ratings have dropped for three weeks, you asked him for more minutes last month, his trust in
you is already low after a broken promise from the other direction, and a match against the league leaders is
on Saturday. The conversation you get is his decision to confront you, and the options you have are the
real ways a player can respond. What he does next depends on his personality, your history with him,
your standing in the dressing room and what else is going on at the club. An AI midfielder in the same
situation would have had the same meeting.

If a moment could only have happened because the game wanted something interesting to happen to the
human, it is wrong.

## 7. How we judge every feature

1. Would it still work with no human player at all?
2. Would it happen to an AI-controlled person in the same circumstances?
3. Is it caused by persistent world state, or invented to entertain?
4. Could it end in several plausible ways depending on the people and circumstances involved?

A feature must pass all four. Details are in the checklist document.
