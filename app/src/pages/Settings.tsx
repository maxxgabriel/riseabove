import { dateIn } from "../format";
import { setSettings, useSettings } from "../settings";
import { act, notify, useStatus } from "../store";
import { Field, Section, Segmented, Switch } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

export function Settings() {
  usePageTitle("Settings");
  const s = useSettings();
  const st = useStatus();
  const inhabiting = st.perspective?.mode === "inhabit";
  const world = st.settings;
  const sample = 20705; // a fixed date so the choices are comparable
  const setWorld = async (patch: Record<string, unknown>) => {
    try {
      await act("settings.set", patch);
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    }
  };
  return (
    <div className="page narrow">
      <PageHead title="Settings" sub="Changes apply straight away." />

      <Section title="Appearance">
        <div className="card form">
          <Field label="Theme">
            <Segmented label="Theme" value={s.theme} onChange={(v) => setSettings({ theme: v })} options={[{ id: "system", label: "Match my system" }, { id: "light", label: "Light" }, { id: "dark", label: "Dark" }]} />
          </Field>
          <Field label="Density" hint="How much fits on screen. Tables and lists follow this.">
            <Segmented label="Density" value={s.density} onChange={(v) => setSettings({ density: v })} options={[{ id: "compact", label: "Compact" }, { id: "comfortable", label: "Comfortable" }, { id: "spacious", label: "Spacious" }]} />
          </Field>
          <Field label={`Text size: ${s.textScale}%`}>
            <input type="range" min={85} max={140} step={5} value={s.textScale} onChange={(e) => setSettings({ textScale: Number(e.target.value) })} aria-valuetext={`${s.textScale} percent`} />
          </Field>
          <Switch checked={s.reduceMotion} onChange={(v) => setSettings({ reduceMotion: v })} label="Reduce motion" hint="Turns off transitions and loading shimmer." />
        </div>
      </Section>

      <Section title="Formats">
        <div className="card form">
          <Field label="Dates">
            <Segmented
              label="Date format"
              value={s.dateStyle}
              onChange={(v) => setSettings({ dateStyle: v })}
              options={(["short", "iso", "us"] as const).map((d) => ({ id: d, label: dateIn(d, sample) }))}
            />
          </Field>
          <Field label="Currency symbol" hint="Display only. The simulation has a single unit of account and nothing is converted.">
            <Segmented label="Currency" value={s.currency} onChange={(v) => setSettings({ currency: v })} options={["£", "€", "$", "¥"].map((c) => ({ id: c, label: c }))} />
          </Field>
        </div>
      </Section>

      {world && (
        <Section title="This world" aside={st.name}>
          <div className="card form">
            <Switch
              checked={world.conceal_mine}
              onChange={(v) => setWorld({ conceal_mine: v })}
              label="Keep my team's results hidden"
              hint={inhabiting ? "Scores of your team's matches stay hidden, in tables too, until you reveal them." : "Applies when you inhabit a player."}
            />
            <div className="field">
              <span className="field-label">Stop advancing time when</span>
              <Switch checked={world.stops.decisions} onChange={(v) => setWorld({ stops: { decisions: v } })} label="A decision needs me" hint="An offer or question arrives for the person you inhabit." />
              <Switch checked={world.stops.matches} onChange={(v) => setWorld({ stops: { matches: v } })} label="My team has a match" hint="Stops the day before, and again once it has been played." />
              <Switch checked={world.stops.major} onChange={(v) => setWorld({ stops: { major: v } })} label="Something major happens to me" hint="An injury, a transfer, a sacking, retirement." />
            </div>
          </div>
        </Section>
      )}

      <Section title="Saving">
        <div className="card form">
          <Switch checked={s.autosave} onChange={(v) => setSettings({ autosave: v })} label="Autosave after time passes" hint="Keeps a copy named “autosave” every few minutes while you advance. It never replaces your own saves." />
        </div>
      </Section>
    </div>
  );
}
