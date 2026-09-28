import { useState } from "react";
import { call, type ApiError } from "../api";
import { dateLong, plural } from "../format";
import { navigate } from "../router";
import { act, getStatus, notify, useStatus } from "../store";
import { Icon } from "../ui/Icon";
import { Button, Dialog, Menu, Progress, type MenuItem } from "../ui/ui";

const DAY = 86_400_000;

async function start(req: Record<string, unknown>) {
  try {
    // "Since you last looked" is measured from the moment you let time pass.
    if (getStatus().perspective?.mode === "inhabit") await call("me.viewed").catch(() => undefined);
    await act("advance.start", req);
  } catch (e) {
    notify({ tone: "neg", text: (e as ApiError).message });
  }
}

export function useAdvance() {
  const st = useStatus();
  const inhabiting = st.perspective?.mode === "inhabit";
  return {
    running: st.job.running,
    primary: () => start(inhabiting ? { mode: "until_event", max_days: 60 } : { mode: "days", n: 1 }),
    days: (n: number) => start({ mode: "days", n }),
    untilEvent: () => start({ mode: "until_event", max_days: 120 }),
    untilMatch: () => start({ mode: "until_match" }),
    untilDate: (d: number) => start({ mode: "until_date", date: d }),
    stop: () => void call("advance.stop").catch(() => undefined),
    inhabiting,
  };
}

export function AdvanceControl() {
  const st = useStatus();
  const adv = useAdvance();
  const [dateOpen, setDateOpen] = useState(false);
  const [picked, setPicked] = useState("");
  const today = st.date ?? 0;
  const job = st.job;

  if (job.running) {
    const total = job.days_total ?? null;
    return (
      <div className="advance-run" role="status" aria-live="polite">
        {total != null ? <Progress value={job.days_done} max={total} label={job.label} /> : <span className="spinner" style={{ width: 14, height: 14 }} />}
        <span className="txt">{total != null ? `${job.days_done} of ${plural(total, "day")}` : `${plural(job.days_done, "day")}`}</span>
        <Button size="sm" icon="stop" onClick={adv.stop} disabled={job.stop_requested}>
          {job.stop_requested ? "Stopping" : "Stop"}
        </Button>
      </div>
    );
  }

  const items: MenuItem[] = [
    ...(adv.inhabiting
      ? ([
          { label: "Until something needs me", hint: "Stops at a decision, match or big news", icon: "play", onSelect: adv.untilEvent },
          { label: "Until my next match", icon: "pitch", onSelect: adv.untilMatch },
          { kind: "sep" },
        ] as MenuItem[])
      : []),
    { label: "1 day", onSelect: () => adv.days(1) },
    { label: "1 week", onSelect: () => adv.days(7) },
    { label: "2 weeks", onSelect: () => adv.days(14) },
    { label: "1 month", onSelect: () => adv.days(30) },
    { label: "3 months", onSelect: () => adv.days(91) },
    { label: "6 months", onSelect: () => adv.days(183) },
    { label: "1 year", onSelect: () => adv.days(365) },
    { kind: "sep" },
    { label: "Until a date…", icon: "calendar", onSelect: () => setDateOpen(true) },
  ];

  const min = new Date((today + 1) * DAY).toISOString().slice(0, 10);
  const target = picked ? Math.round(Date.parse(`${picked}T00:00:00Z`) / DAY) : null;
  const valid = target != null && target > today && target - today <= 3660;

  return (
    <>
      <div className="advance">
        <Button variant="primary" icon="play" onClick={adv.primary} title={adv.inhabiting ? "Continue until something needs you (Ctrl+Enter)" : "Advance one day (Ctrl+Enter)"}>
          {adv.inhabiting ? "Continue" : "Next day"}
        </Button>
        <Menu items={items} align="end" label="Advance time">
          {({ setRef, toggle, open }) => (
            <Button variant="primary" ref={setRef} onClick={toggle} aria-label="More ways to advance time" aria-haspopup="menu" aria-expanded={open}>
              <Icon name="down" size={14} />
            </Button>
          )}
        </Menu>
      </div>
      <Dialog
        open={dateOpen}
        onClose={() => setDateOpen(false)}
        title="Advance until a date"
        width={380}
        footer={
          <>
            <Button variant="ghost" onClick={() => setDateOpen(false)}>Cancel</Button>
            <Button
              variant="primary"
              disabled={!valid}
              onClick={() => {
                setDateOpen(false);
                if (target != null) void adv.untilDate(target);
              }}
            >
              Advance
            </Button>
          </>
        }
      >
        <label className="field">
          <span className="field-label">Date</span>
          <input type="date" min={min} value={picked} onChange={(e) => setPicked(e.target.value)} autoFocus />
          <span className="hint">
            {target != null && target > today ? `${plural(target - today, "day")} from now, ${dateLong(target)}.` : `Today is ${dateLong(today)}.`}
          </span>
        </label>
      </Dialog>
    </>
  );
}

export function goToInhabit() {
  navigate("/inhabit");
}
