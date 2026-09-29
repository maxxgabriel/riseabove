import { useState } from "react";
import { act, notify, useStatus } from "../store";
import { navigate } from "../router";
import { Button, Dialog, Switch } from "../ui/ui";

/** Confirm living as someone. Switching keeps the world as it is; only the point of view changes. */
export function InhabitDialog({ open, onClose, person }: { open: boolean; onClose: () => void; person: { id: number; name: string; short: string } }) {
  const st = useStatus();
  const [conceal, setConceal] = useState(true);
  const [working, setWorking] = useState(false);
  const already = st.perspective?.mode === "inhabit" ? st.perspective : null;
  const go = async () => {
    setWorking(true);
    try {
      await act("persp.inhabit", { person: person.id });
      await act("settings.set", { conceal_mine: conceal });
      onClose();
      navigate("/today");
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    } finally {
      setWorking(false);
    }
  };
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={`Live as ${person.name}?`}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={go} disabled={working}>Inhabit</Button>
        </>
      }
    >
      <p>You will see the world as {person.short} does: their messages, their contract and condition, and only what their club would tell them about others. Decisions that reach them become yours to make.</p>
      {already && <p className="muted">You are currently {already.name}. Switching changes nothing in the world, and you can go back to observing at any time.</p>}
      <Switch checked={conceal} onChange={setConceal} label="Keep my team's results hidden" hint="Scores of their matches stay hidden until you choose to reveal them. Tables adjust to match." />
    </Dialog>
  );
}
