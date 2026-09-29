import { useEffect, useState, type ReactNode } from "react";
import { act, notify, useApi } from "../store";
import type { Named } from "../types";
import { Button, Dialog, Field, Menu } from "../ui/ui";

export interface Keyed {
  key: string;
  label: string;
}
export interface Options {
  meet_with: { role: string; who: Named }[];
  topics: Keyed[];
  tones: Keyed[];
  stances: Keyed[];
  posts: Keyed[];
  lifestyles: Keyed[];
  routine_budget: number;
  agents: { id: number; person: Named; base: string; clients: number; reputation: number }[];
  courses: { key: string; label: string; cost: number; effort: number; requires: string | null; done: boolean; studying: boolean }[];
  helpers: { key: string; label: string; base_cost: number; hired: number | null }[];
  careers: Keyed[];
  roles: Keyed[];
  nations: { id: number; name: string }[];
}

export const useOptions = (enabled = true) => useApi<Options>(enabled ? "me.options" : null);

/** Queue something for the inhabited person to do. The world acts on it when the day ends. */
export async function queueAction(action: string, args: Record<string, unknown> = {}): Promise<boolean> {
  try {
    const r = await act<{ text: string; applies: string }>("me.act", { action, ...args });
    notify({ tone: "pos", text: `Queued: ${r.text}. The world acts on it when the day ends.` });
    return true;
  } catch (e) {
    notify({ tone: "neg", text: (e as Error).message });
    return false;
  }
}

function Select({ value, onChange, options, label }: { value: string; onChange: (v: string) => void; options: Keyed[]; label: string }) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value)} aria-label={label}>
      {options.map((o) => (
        <option key={o.key} value={o.key}>{o.label[0].toUpperCase() + o.label.slice(1)}</option>
      ))}
    </select>
  );
}

// ---- ask to meet ------------------------------------------------------------------------------

export function MeetDialog({ open, who, onClose, topic: preset }: { open: boolean; who: Named | null; onClose: () => void; topic?: string }) {
  const opts = useOptions(open);
  const [topic, setTopic] = useState(preset ?? "");
  const [tone, setTone] = useState("calm");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (open) setTopic(preset ?? opts.data?.topics[0]?.key ?? "");
  }, [open, preset, opts.data]);
  if (!who) return null;
  const go = async () => {
    setBusy(true);
    const ok = await queueAction("meet", { with: who.id, topic, tone });
    setBusy(false);
    if (ok) onClose();
  };
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={`Ask ${who.name} for a word`}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !topic} onClick={go}>Ask for a meeting</Button>
        </>
      }
    >
      <p className="muted">They decide whether to see you. How it goes depends on how they see you and the tone you take. Either way it is remembered.</p>
      {opts.data && (
        <>
          <Field label="What about?">
            <Select value={topic} onChange={setTopic} options={opts.data.topics} label="Subject" />
          </Field>
          <Field label="How will you put it?">
            <Select value={tone} onChange={setTone} options={opts.data.tones} label="Tone" />
          </Field>
        </>
      )}
    </Dialog>
  );
}

// ---- post online ------------------------------------------------------------------------------

export function PostDialog({ open, onClose, replyTo, quoteOf, about, context }: { open: boolean; onClose: () => void; replyTo?: number; quoteOf?: number; about?: Named | null; context?: ReactNode }) {
  const opts = useOptions(open);
  const [concept, setConcept] = useState("statement");
  const [busy, setBusy] = useState(false);
  const go = async () => {
    setBusy(true);
    const ok = await queueAction("post", { concept, about: about?.id, reply_to: replyTo, quote_of: quoteOf });
    setBusy(false);
    if (ok) onClose();
  };
  const verb = replyTo != null ? "Reply" : quoteOf != null ? "Quote" : "Post";
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={`${verb} from your account`}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={go}>{verb} tomorrow</Button>
        </>
      }
    >
      {context}
      <p className="muted">You choose what you are doing, not the words. The post is written from your voice and what you know, and goes out when the day ends.</p>
      {opts.data && (
        <>
          <Field label="What kind of post?">
            <Select value={concept} onChange={setConcept} options={opts.data.posts} label="Kind of post" />
          </Field>
          {about && <p className="hint">About {about.name}.</p>}
        </>
      )}
    </Dialog>
  );
}

// ---- speak to the press ------------------------------------------------------------------------

export function SpeakDialog({ open, onClose, about }: { open: boolean; onClose: () => void; about: Named }) {
  const opts = useOptions(open);
  const [stance, setStance] = useState("praise");
  const [busy, setBusy] = useState(false);
  const go = async () => {
    setBusy(true);
    const ok = await queueAction("press", { about: about.id, stance });
    setBusy(false);
    if (ok) onClose();
  };
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={`Speak to the press about ${about.name}`}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={go}>Speak tomorrow</Button>
        </>
      }
    >
      <p className="muted">A journalist gets your line the next day and may print it. What you say is on the record and can be quoted back to you.</p>
      {opts.data && (
        <Field label="Your line">
          <Select value={stance} onChange={setStance} options={opts.data.stances} label="Stance" />
        </Field>
      )}
    </Dialog>
  );
}

// ---- confirm before doing something that is hard to take back -----------------------------------------------

export function ConfirmAction({
  label,
  title,
  children,
  action,
  args,
  danger,
  disabled,
  confirmLabel,
  size = "md",
  variant,
}: {
  label: string;
  title: string;
  children: ReactNode;
  action: string;
  args?: Record<string, unknown>;
  danger?: boolean;
  disabled?: boolean;
  confirmLabel?: string;
  size?: "sm" | "md";
  variant?: "default" | "primary" | "ghost" | "danger";
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const go = async () => {
    setBusy(true);
    const ok = await queueAction(action, args ?? {});
    setBusy(false);
    if (ok) setOpen(false);
  };
  return (
    <>
      <Button size={size} variant={variant ?? (danger ? "danger" : "default")} disabled={disabled} onClick={() => setOpen(true)}>{label}</Button>
      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title={title}
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>Not now</Button>
            <Button variant={danger ? "danger" : "primary"} disabled={busy} onClick={go}>{confirmLabel ?? label}</Button>
          </>
        }
      >
        {children}
      </Dialog>
    </>
  );
}

/** A button that opens the "ask for a word" dialog for one person. */
export function TalkButton({ who, label = "Ask for a word", size = "sm", topic }: { who: Named; label?: string; size?: "sm" | "md"; topic?: string }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button size={size} onClick={() => setOpen(true)}>{label}</Button>
      <MeetDialog open={open} who={who} onClose={() => setOpen(false)} topic={topic} />
    </>
  );
}

/** What you can do about another person, gathered in one menu. */
export function PersonActions({ who, canMentor }: { who: Named; canMentor?: boolean }) {
  const [dialog, setDialog] = useState<"meet" | "post" | "speak" | null>(null);
  return (
    <>
      <Menu
        align="end"
        label={`Do something about ${who.name}`}
        items={[
          { label: "Ask for a word…", icon: "mail", onSelect: () => setDialog("meet") },
          { label: "Post about them…", icon: "pulse", onSelect: () => setDialog("post") },
          { label: "Speak to the press about them…", icon: "star", onSelect: () => setDialog("speak") },
          ...(canMentor ? [{ label: "Offer to mentor them", icon: "person" as const, onSelect: () => void queueAction("mentor", { person: who.id }) }] : []),
        ]}
      >
        {({ setRef, toggle, open }) => (
          <Button ref={setRef as never} onClick={toggle} aria-haspopup="menu" aria-expanded={open} icon="person">Do something</Button>
        )}
      </Menu>
      <MeetDialog open={dialog === "meet"} who={who} onClose={() => setDialog(null)} />
      <PostDialog open={dialog === "post"} about={who} onClose={() => setDialog(null)} />
      <SpeakDialog open={dialog === "speak"} about={who} onClose={() => setDialog(null)} />
    </>
  );
}
