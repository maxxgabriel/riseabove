import { useState } from "react";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, Button, Dialog, Meter } from "../ui/ui";
import { ConfirmAction, TalkButton, queueAction, useOptions } from "./Actions";
import { Dt, EntityLink } from "./links";
import { bandFill, duration } from "../format";
import type { Band } from "../contract.generated";

interface AgentResp {
  player: boolean;
  agent: null | {
    id: number;
    who: Named;
    fee_pct: number;
    since: number;
    until: number;
    satisfaction: Band;
    reputation: number;
    clients: number;
    base: string;
  };
}

/** Your agent, or the agents you could ask. */
export function AgentPanel({ today }: { today: number }) {
  const q = useApi<AgentResp>("me.agent");
  const [choosing, setChoosing] = useState(false);
  const opts = useOptions(choosing);
  if (!q.data?.player) return null;
  const a = q.data.agent;
  return (
    <div className="card form">
      {a ? (
        <>
          <div className="talk-head">
            <div>
              <strong><EntityLink r={a.who}>{a.who.name}</EntityLink></strong>
              <div className="hint">Based in {a.base} · {a.clients} clients</div>
            </div>
            <Badge tone={bandFill(a.satisfaction) >= 60 ? "pos" : bandFill(a.satisfaction) >= 35 ? "warn" : "neg"}>{a.satisfaction.label}</Badge>
          </div>
          <div className="meter-row"><span>How happy they are with you</span><Meter value={bandFill(a.satisfaction)} label={a.satisfaction.label} /></div>
          <p className="hint">Takes {a.fee_pct}% of what you earn from new deals. Signed <Dt d={a.since} />, until <Dt d={a.until} /> ({duration(a.until - today)}).</p>
          <div className="formfoot">
            <TalkButton who={a.who} label="Ask for a word" topic="agent_review" size="md" />
            <ConfirmAction label="Part ways" title={`Part ways with ${a.who.name}?`} action="drop_agent" danger>
              <p>You would be on your own until someone else agrees to represent you.</p>
            </ConfirmAction>
          </div>
        </>
      ) : (
        <>
          <p className="muted">You do not have an agent. An agent hears about interest before you do and handles the details of talks, for a share of what you earn.</p>
          <div className="formfoot"><Button onClick={() => setChoosing(true)}>Look for an agent</Button></div>
        </>
      )}
      <Dialog open={choosing} onClose={() => setChoosing(false)} title="Ask an agent to represent you" width={520}>
        <p className="muted">Better-known agents take on fewer clients. They decide whether to take you.</p>
        {opts.data && (
          <ul className="rows">
            {[...opts.data.agents].sort((x, y) => y.reputation - x.reputation).slice(0, 12).map((ag) => (
              <li key={ag.id}>
                <div>
                  <div>{ag.person.name}</div>
                  <div className="hint">{ag.base} · {ag.clients} clients · reputation {ag.reputation}</div>
                </div>
                <Button size="sm" onClick={async () => { if (await queueAction("hire_agent", { agent: ag.id })) setChoosing(false); }}>Ask</Button>
              </li>
            ))}
          </ul>
        )}
      </Dialog>
    </div>
  );
}
