# 10 — Life Simulation (off the pitch)

The life layer makes the career *personal* without turning into a separate game. Its effects on football are **bounded and symmetric**: AI players get the same effects from a statistical life model.

## 1. Time budget

Each week has ~112 waking hours. Fixed blocks come from the club schedule (training, travel, matches, media duties). The rest is allocated by the player (or default routine):

| Activity | Effects |
|----------|---------|
| Rest / sleep extra | Recovery ↑, stress ↓ |
| Recovery work (pool, massage) | Condition ↑, fatigue debt ↓ |
| Extra individual training | Development ↑, load ↑ |
| Education / study | Grades, qualifications, languages |
| Family time | Family relationship ↑, stress ↓ |
| Partner time | Relationship ↑ |
| Friends / social | Friendship ↑, happiness ↑ |
| Hobbies | Happiness ↑, stress ↓ (hobby catalogue: gaming, music, reading, golf, cooking, fishing, art…) |
| Media / sponsor duties | Income, reputation, fatigue small |
| Community / charity | Public reputation ↑, fulfilment ↑ |
| Language classes | Language proficiency ↑ (adaptation) |
| Coaching badges (late career) | Post-career readiness |

Routines: saved presets ("match week", "injury rehab", "off-season"); the week auto-applies if untouched.

## 2. Well-being model

```
wellbeing = f(sleep_quality, stress, happiness, loneliness, fulfilment, physical_health)
stress    += career_pressure + media_pressure + relationship_conflicts + financial_worries + relocation_shock + injury_frustration
stress    −= rest + hobbies + support_network + psychologist_sessions + success
```

Bounded effects on football: sleep/wellbeing → recovery rate (±10%), Concentration/Decisions effectiveness in matches (±3%), training rating (±0.3), injury hazard (±10%). Same function for AI players with statistically generated life inputs based on their personality, age, family status and location.

## 3. Education

| Age | System |
|-----|--------|
| 8–16 | School: grades (0–100 per subject cluster), attendance, exam years; academy requires minimum attendance; poor grades can cause conflict with parents |
| 16–18 | Academy education programme (vocational/academic); qualifications earned |
| 18+ | Optional: distance degree, business courses, languages, coaching badges |

Education pays off post-career (options, income) and slightly in media handling/adaptability. Dropping education is allowed with consequences (family relationship, post-career options).

## 4. Family

- Family members are Persons (parents, siblings, grandparents; later partner and children), with personality, occupation, location, health.
- Family relationship scores; family events (birthdays, weddings, illnesses, sibling milestones, parents moving) create calendar requests.
- Parents' influence is high when young (contract decisions, agent choice, education), shrinking with age.
- Family football history can create expectations or support.
- Children (if the player chooses to start a family with a partner): schedule needs, school considerations affecting relocation willingness.

## 5. Relationships (romantic & social)

- Meeting people through social activities, hometown, teammates' circles; dating is a light, respectful system: compatibility, time investment, communication choices. No explicit content.
- Partner is a Person with a career, location ties, language, ambitions. Partner's willingness to relocate is a real input to transfer decisions (08 §4 `w_life`).
- Long-distance strain, moving together, engagement, marriage, separation — all possible, driven by time and choices. The game never forces a relationship; single life is equally valid.
- Friendships: childhood friends, teammates, neighbours; friends can become agents/staff/business partners or bad influences (professionalism effect).

## 6. Housing & relocation

- Housing tiers (shared academy digs, family home, rented flat, bought house, luxury); location choice relative to training ground (commute time affects time budget).
- Relocation: moving country → language shock, culture adaptation (Adaptability), homesickness (loyalty/family), partner/family decisions (come along, stay, visit).
- Club relocation support (hosted by player liaison officer if club has one).

## 7. Personal finances

| Component | Detail |
|-----------|--------|
| Income | Wages, bonuses, signing fees, loyalty bonuses, image rights, sponsorships, media fees, prize money, investments |
| Taxes | Per-nation simplified tax model (progressive bands, image-rights treatment, special regimes as data) |
| Deductions | Agent fees, pension contributions (per nation), union fees |
| Expenses | Housing, lifestyle tier, family support, travel, staff (personal trainer, chef), charity |
| Assets | Savings, property, investment funds (risk/return profiles), business ventures (post-career) |
| Liabilities | Mortgages, loans |
| Risk events | Poor investments, lifestyle creep, divorce settlement (if applicable), advisor fraud risk mitigated by choosing reputable advisors |

Monthly statement UI; financial advisor option; net-worth history chart. **No gambling mechanics.** No real-money transactions.

## 8. Sponsorship & commercial

- Offers depend on reputation, market, image (controversy), social media following.
- Contract terms: duration, appearances required (time budget), exclusivity (conflicts with club sponsors rule).
- Boot deals as a prestige ladder.

## 9. Identity & personality growth

- Protagonist personality can drift slowly from choices (e.g. consistent professional choices raise Professionalism perception and actual hidden value within bounded rates — same rate caps as AI players' mentoring drift).
- Values/priorities profile (money, trophies, family, legacy, fame) shaped by choices; used by the default AiMind so autopilot "acts like you".

## 10. Health & mind

- Physical health (05), nutrition & sleep habits (lifestyle choices).
- Mental health: stress, anxiety moments after big failures, confidence crises; support options: talking to family/friends, club psychologist, time off. Presented supportively (see §12).
- Medical events off-pitch (illness), family health events (visiting family).

## 11. Life events catalogue (examples)

Birthday, graduation, driving test, first car, moving out, sibling's wedding, grandparent's illness, becoming a parent, charity invitation, hometown event, school reunion, friend's business opportunity, fan meeting, award ceremony, holiday planning, national holiday, religious/cultural festivals (data pack, respectful), language exam, citizenship ceremony.

## 12. Content guidelines

- Sensitive themes (mental health, injury-ended careers, bereavement, discrimination faced) are handled with care: no gamified suffering, supportive options always available, optional content toggles.
- Harmful behaviours (substance abuse, gambling, violence) are not playable loops. At most they appear as world rules and generic disciplinary outcomes for AI persons in news, without glamorisation or detail.
- Discrimination in football (e.g. abuse from crowds) may be referenced in news with anti-discrimination responses and support; never as a mechanic the player uses.
- Romance is non-explicit.

## 13. AI players' statistical life model

For fairness, each AI player has a compact life state updated monthly: family status, relocation willingness, stress level, lifestyle quality, sponsorship income — sampled from personality/age/nation distributions and influenced by events (transfer abroad, injury). These produce the same bounded football effects the protagonist's detailed life produces.
