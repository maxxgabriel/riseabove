# 16 — Edge-Case Catalogue

Each entry: the case, and the required behaviour. Every case gets at least one scenario test.

## A. Calendar & time

| # | Case | Required behaviour |
|---|------|--------------------|
| A1 | Player born 29 February | Age increments on 1 March in non-leap years (configurable per nation) |
| A2 | Transfer between leagues with different season calendars (spring–autumn ↔ autumn–spring) | Stats split by competition season; contract dates independent; registration windows of the receiving nation apply |
| A3 | Split seasons (apertura/clausura) | Two champions + aggregate table for relegation/continental spots |
| A4 | International window overlaps domestic matches in a league that doesn't pause | Club plays without called-up players; fatigue/travel applied |
| A5 | Matches postponed repeatedly | Rescheduler finds dates; if impossible, rules decide (awarded result / season cut-off) |
| A6 | Season curtailed (force majeure event, configurable, off by default) | Points-per-game standings per rules |
| A7 | Winter break with midseason friendlies/tours | Travel and fatigue handled; no registration changes outside window |
| A8 | Player turns 18 mid-window | Minors rule stops applying from the birthday date |
| A9 | Contract expires on a match day | Expiry at midnight after the last day; player can play that day |

## B. Registration & eligibility

| # | Case | Required behaviour |
|---|------|--------------------|
| B1 | Cup-tied player joins mid-cup | Ineligible for that competition this season |
| B2 | Player registered for 3 clubs in a season | Cannot join a 4th / cannot play for a 3rd per rule profile |
| B3 | Loanee facing parent club | Per loan clause/rules |
| B4 | Homegrown quota breached by injuries | Squad list stays legal (registration counts list, not availability) |
| B5 | Foreigner limit on the pitch (not in squad) | Selector enforces on-pitch limit including subs |
| B6 | Goalkeeper crisis (all GKs injured) | Emergency GK signing if rules allow; else outfield player in goal |
| B7 | Dual nationality changes foreigner status | Foreigner status re-evaluated on citizenship date |
| B8 | Work permit refused after agreed transfer | Deal collapses or loan to a third nation (if the club chooses) |
| B9 | Player suspended across competitions | Served per rules (competition-specific vs all) |
| B10 | Suspension carried over into new season / new club | Per rule profile |
| B11 | Youth player too old for youth competition mid-season | Cut-off date rule |
| B12 | International switch attempted after competitive senior cap | Rules engine refuses with reason |
| B13 | Player eligible through grandparent only after document process | Eligibility effective after paperwork delay event |
| B14 | Naturalisation requires residency years interrupted by a loan abroad | Residency clock rules (reset/pause) per nation profile |

## C. Contracts & market

| # | Case | Required behaviour |
|---|------|--------------------|
| C1 | Release clause triggered by an unwanted club | Player still must agree personal terms |
| C2 | Relegation wage-drop clause | Applies on relegation confirmation date |
| C3 | Promotion clause while on loan | Clause belongs to parent-club contract; loan club promotion does not trigger unless specified |
| C4 | Club goes into administration mid-contract | Wages may be unpaid → player gains termination right after N months; possible fire sale |
| C5 | Club liquidated | All contracts void; players become free agents immediately (outside window rules apply) |
| C6 | Player signs pre-contract then gets seriously injured | New club may still honour or renegotiate per rules; medical outcome logged |
| C7 | Loan recall during injury crisis | Only if recall clause and window/rules allow |
| C8 | Loan with obligation to buy triggered by appearances | Automatic permanent deal on threshold |
| C9 | Sell-on clause when player moves on | Fee share paid to prior club |
| C10 | Transfer fee in instalments and selling club dissolves | Remaining instalments to administrators or written off per rule |
| C11 | Player refuses to report for training (strike) | Fines, suspension, relationship damage; possible termination by club |
| C12 | Mutual termination with payout | Remaining wages compensation negotiated |
| C13 | Agent representation expires during negotiation | Negotiation paused; player negotiates alone or picks new agent |
| C14 | Minimum wage changes with legislation | Contracts below minimum auto-raised |
| C15 | Player under 18 offered a contract longer than allowed | Rules engine caps or rejects |
| C16 | Two clubs agree fees for the same player simultaneously | Player chooses; the other deal lapses |
| C17 | Deadline-day paperwork late | Deal fails or completes per "deal sheet" rule toggle |
| C18 | Currency devaluation affects wage value | Contract in club currency; personal finances reflect conversion |
| C19 | Player retires with years left on contract | Contract terminated; possible repayment of signing fees per terms |
| C20 | Player comes out of retirement | Re-registration as free agent; fitness/sharpness very low |

## D. Match

| # | Case | Required behaviour |
|---|------|--------------------|
| D1 | Abandoned match (weather/crowd) | Rules: replay / result stands / awarded |
| D2 | Team reduced below 7 players | Match abandoned, result awarded |
| D3 | Own goal attribution | Recorded as own goal; no scorer credit |
| D4 | Goalkeeper scores | Counted; commentary/records special case |
| D5 | Penalty retaken (encroachment/GK off line) | Retake event recorded |
| D6 | Concussion substitute after all subs used | Allowed per rules; extra sub counted |
| D7 | Player sent off from the bench | Counts as red card; no reduction on pitch |
| D8 | Extra time + penalties in two-legged tie | Away-goals rule toggle per season |
| D9 | Protagonist injured in warm-up | Replaced in line-up; counts as not played |
| D10 | Player plays multiple positions in one match | positions_played timeline recorded; familiarity grows per minutes in each |
| D11 | Match forfeited due to ineligible player | Result overturned post-match (rules), stats kept or voided per rule |

## E. Health

| # | Case | Required behaviour |
|---|------|--------------------|
| E1 | Injury during international duty | Club informed; compensation rules (optional); fatigue/relationship effects |
| E2 | Re-injury during rehab | Setback extends timeline; recurrence risk ↑ |
| E3 | Career-threatening injury | Medical advice flow; player chooses to continue rehab or retire; handled sensitively |
| E4 | Illness outbreak affects whole squad | Postponement request per rules if too few players |
| E5 | Player plays through knock and aggravates | Longer injury; relationship with medical staff/manager depends on who decided |

## F. Club & world

| # | Case | Required behaviour |
|---|------|--------------------|
| F1 | Manager sacked the day before the match | Caretaker selects team; perceptions from staff notes |
| F2 | Club merger | Squads merged; contracts continue; surplus released per rules |
| F3 | League expansion/contraction | Transitional promotion/relegation counts |
| F4 | Promotion blocked by licensing | Next eligible club promoted |
| F5 | Points deduction mid-season | Table updates; relegation risk; board reaction |
| F6 | B-team would be promoted to same tier as A-team | Promotion skips B-team |
| F7 | Club with zero eligible manager candidates | Caretaker from staff; long-term search widens criteria |
| F8 | Player becomes manager of own club (player-coach) | Dual role contract; selection AI must allow self-selection neutrally |
| F9 | Former teammate becomes your manager | Relationship history carried; affects trust |
| F10 | Takeover changes budget mid-window | Budgets revised; ongoing negotiations re-evaluated |

## G. Protagonist & life

| # | Case | Required behaviour |
|---|------|--------------------|
| G1 | User ignores all decisions for weeks | Defaults apply at deadlines; world continues |
| G2 | Protagonist released from academy at 16 | Grassroots/semi-pro path continues; trials possible |
| G3 | Protagonist unattached for a long period | Training alone (sharpness decays), trial invites by agent, lower leagues interest |
| G4 | Partner refuses to relocate | Real input to decision; long-distance or separation paths |
| G5 | Family emergency during match week | Request for leave; manager response by personality |
| G6 | School exam on match day (youth) | Club/academy policy decides; conflict choice |
| G7 | Protagonist's sibling is also a footballer | Normal Person in world; relationship & "brothers at same club" story possible |
| G8 | Protagonist switches nationality allegiance | Rules engine; media/fan reaction both nations |
| G9 | Protagonist asks to be transfer-listed then changes mind | Club may delist; trust penalties |
| G10 | Save loaded on a newer version | Migration; world continues; release notes of changed rules |
| G11 | Protagonist's former youth club merges or folds | History preserved; solidarity payments route to successor or lapse |

## H. Data & persistence

| # | Case | Required behaviour |
|---|------|--------------------|
| H1 | Mod pack removes a competition mid-save | Blocked: mods bound at save creation unless migration script provided |
| H2 | Name collisions in generated world | Resolved at generation |
| H3 | Disk full during autosave | Temp file discarded; previous save intact; warning shown |
| H4 | Crash mid-day | Resume from last autosave; decision log intact |
