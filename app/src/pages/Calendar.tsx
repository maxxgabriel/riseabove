import type { MeCalendarView } from "../contract.generated";
import { useState } from "react";
import { href } from "../router";
import { monthName, toDate } from "../format";
import { useApi, useStatus } from "../store";
import { Button, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type CalResp = MeCalendarView;

const mondayIndex = (d: number) => (d + 3) % 7; // 1970-01-01 was a Thursday

export function Calendar() {
  usePageTitle("Calendar");
  const [offset, setOffset] = useState(0);
  const today = useStatus().date ?? 0;
  // Six weeks starting on the Monday of the week that includes today, shifted by whole weeks.
  const start = today - mondayIndex(today) + offset * 7;
  const q = useApi<CalResp>("me.calendar", { from: start, to: start + 41 });
  return (
    <div className="page">
      <PageHead
        title="Calendar"
        sub="Matches, training and dates that matter to you."
        actions={
          <>
            <Button size="sm" onClick={() => setOffset((o) => o - 6)}>Earlier</Button>
            <Button size="sm" onClick={() => setOffset(0)} disabled={offset === 0}>Today</Button>
            <Button size="sm" onClick={() => setOffset((o) => o + 6)}>Later</Button>
          </>
        }
      />
      <Async q={q}>
        {(c) => {
          const byDate = new Map(c.days.map((d) => [d.date, d.entries]));
          const weeks = Array.from({ length: 6 }, (_, w) => Array.from({ length: 7 }, (_, i) => start + w * 7 + i));
          const first = toDate(start);
          const last = toDate(start + 41);
          const title = first.getUTCMonth() === last.getUTCMonth() ? `${monthName(first.getUTCMonth(), true)} ${first.getUTCFullYear()}` : `${monthName(first.getUTCMonth(), true)} to ${monthName(last.getUTCMonth(), true)} ${last.getUTCFullYear()}`;
          return (
            <Section title={title}>
              <div className="cal" role="grid" aria-label="Calendar">
                <div className="cal-head" role="row">
                  {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((d) => <div key={d} role="columnheader">{d}</div>)}
                </div>
                {weeks.map((w, wi) => (
                  <div key={wi} className="cal-week" role="row">
                    {w.map((d) => {
                      const entries = byDate.get(d) ?? [];
                      const t = toDate(d);
                      const isToday = d === c.today;
                      const past = d < c.today;
                      const important = entries.filter((e) => e.kind !== "training" && e.kind !== "rest" && e.kind !== "recovery");
                      const routine = entries.filter((e) => e.kind === "training" || e.kind === "rest" || e.kind === "recovery");
                      return (
                        <div key={d} role="gridcell" className={`cal-day ${isToday ? "today" : ""} ${past ? "past" : ""}`} aria-label={`${t.getUTCDate()} ${monthName(t.getUTCMonth(), true)}`}>
                          <div className="cal-num num">{t.getUTCDate() === 1 || d === start ? `${t.getUTCDate()} ${monthName(t.getUTCMonth())}` : t.getUTCDate()}</div>
                          {important.map((e, i) => (
                            <div key={i} className={`cal-entry k-${e.kind}`} title={`${e.label}${e.sub ? `, ${e.sub}` : ""}. ${e.source}`}>
                              {e.ref ? <a href={href(`/match/${e.ref.id}`)}>{e.label}{e.result ? ` ${e.result}` : ""}</a> : e.label}
                            </div>
                          ))}
                          {routine.length > 0 && important.length === 0 && <div className={`cal-routine k-${routine[0].kind}`} title={routine[0].label}>{routine[0].kind === "rest" ? "Rest" : routine[0].kind === "recovery" ? "Recovery" : ""}</div>}
                        </div>
                      );
                    })}
                  </div>
                ))}
              </div>
              <div className="cal-legend hint">
                <span><i className="dot k-match" /> Match</span>
                <span><i className="dot k-contract" /> Contract</span>
                <span><i className="dot k-window" /> Transfer window</span>
                <span><i className="dot k-decision" /> Answer due</span>
                <span>Plain days are training. Fixtures further ahead appear once they are scheduled.</span>
              </div>
            </Section>
          );
        }}
      </Async>
    </div>
  );
}
