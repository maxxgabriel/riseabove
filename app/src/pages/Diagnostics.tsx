import { fmtInt } from "../format";
import { useApi } from "../store";
import { KeyVal, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Diag {
  version: string;
  revision: number;
  perspective: string;
  timings: { avg_ms: number; worst_ms: number; samples: number; recent: [number, number][] };
  world: Record<string, number | string>;
}

export function Diagnostics() {
  usePageTitle("Diagnostics");
  const q = useApi<Diag>("diagnostics");
  return (
    <div className="page narrow">
      <PageHead title="Diagnostics" sub="For checking that the world is healthy." />
      <Async q={q}>
        {(d) => (
          <>
            <Section title="Speed">
              <div className="card">
                <KeyVal rows={[
                  { k: "Average per simulated day", v: <span className="num">{d.timings.avg_ms.toFixed(1)} ms</span> },
                  { k: "Slowest day", v: <span className="num">{d.timings.worst_ms.toFixed(1)} ms</span> },
                  { k: "Days measured", v: <span className="num">{fmtInt(d.timings.samples)}</span> },
                ]} />
              </div>
            </Section>
            <Section title="World">
              <div className="card">
                <KeyVal rows={Object.entries(d.world).map(([k, v]) => ({ k: k.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase()), v: <span className="num">{typeof v === "number" ? fmtInt(v) : v}</span> }))} />
              </div>
            </Section>
            <Section title="Build">
              <div className="card">
                <KeyVal rows={[{ k: "Version", v: d.version }, { k: "Point of view", v: d.perspective }, { k: "Change counter", v: <span className="num">{fmtInt(d.revision)}</span> }]} />
              </div>
            </Section>
          </>
        )}
      </Async>
    </div>
  );
}
