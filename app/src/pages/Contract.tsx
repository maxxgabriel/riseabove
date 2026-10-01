import { EntityLink, Money, Dt } from "../components/links";
import { duration } from "../format";
import { href } from "../router";
import { useApi, useStatus } from "../store";
import type { Named, Part } from "../types";
import { Parts } from "../components/links";
import { AgentPanel } from "../components/AgentPanel";
import { ConfirmAction } from "../components/Actions";
import { TalkBlock } from "../components/Decision";
import { Empty, KeyVal, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Row {
  label: string;
  money?: number | null;
  date?: number;
  text?: string | null;
}
interface ContractResp {
  has_contract: boolean;
  status?: string;
  club?: Named;
  summary?: { wage: number; end: number; days_left: number; status: string; kind: string };
  terms?: Row[];
  loan?: null | { parent: Named; club: Named; end: number; recall: boolean; wage_share: number; buy_option: number };
  offers?: { id: string; title: string; deadline: number }[];
  history?: { date: number; parts: Part[] }[];
  guaranteed_note?: string;
  talks?: React.ComponentProps<typeof TalkBlock>["t"] | null;
  transfer_request?: number | null;
  listed?: boolean;
}

export function Contract() {
  usePageTitle("Contract");
  const q = useApi<ContractResp>("me.contract");
  const today = useStatus().date ?? 0;
  return (
    <div className="page">
      <Async q={q}>
        {(c) =>
          !c.has_contract ? (
            <>
              <PageHead title="Contract" />
              <Empty title="You do not have a contract" icon="contract">You are {c.status?.toLowerCase() ?? "without a club"}. Offers from clubs arrive in your <a href={href("/messages")}>messages</a>.</Empty>
              <div className="stack" style={{ maxWidth: "36rem", marginTop: "1rem" }}>
                <Section title="Your agent"><AgentPanel today={today} /></Section>
              </div>
            </>
          ) : (
            <>
              <PageHead
                title="Contract"
                sub={c.club && <span>With <EntityLink r={c.club}>{c.club.name}</EntityLink>, {duration(c.summary!.days_left)} remaining.</span>}
              />
              <div className="split">
                <div className="stack">
                  <Section title="Terms">
                    <div className="card">
                      <KeyVal
                        rows={c.terms!.map((r) => ({
                          k: r.label,
                          v: r.text != null && r.text !== "" ? r.text : r.date != null ? <Dt d={r.date} /> : r.money != null ? <Money v={r.money} exact /> : "None",
                        }))}
                      />
                      {c.guaranteed_note && <p className="hint" style={{ marginTop: "0.6rem" }}>{c.guaranteed_note}</p>}
                    </div>
                  </Section>
                  {c.talks && (
                    <Section title="Talks in progress">
                      <TalkBlock t={c.talks} />
                    </Section>
                  )}
                  {c.loan && (
                    <Section title="On loan">
                      <div className="card">
                        <KeyVal
                          rows={[
                            { k: "Owned by", v: <EntityLink r={c.loan.parent}>{c.loan.parent.name}</EntityLink> },
                            { k: "Playing for", v: <EntityLink r={c.loan.club}>{c.loan.club.name}</EntityLink> },
                            { k: "Loan ends", v: <Dt d={c.loan.end} /> },
                            { k: "Can be recalled", v: c.loan.recall ? "Yes" : "No" },
                            { k: "Wages paid by borrower", v: <span className="num">{c.loan.wage_share}%</span> },
                            { k: "Option to buy", v: c.loan.buy_option > 0 ? <Money v={c.loan.buy_option} /> : "None" },
                          ]}
                        />
                      </div>
                    </Section>
                  )}
                </div>
                <aside className="stack">
                  <Section title="Wanting to leave">
                    <div className="card form">
                      {c.listed && <div className="note warn"><span>The club has put you up for transfer.</span></div>}
                      {c.transfer_request != null ? (
                        <>
                          <p>You handed in a transfer request on <Dt d={c.transfer_request} />. The club, and the people who follow the club, know it.</p>
                          <div className="formfoot">
                            <ConfirmAction label="Withdraw the request" title="Withdraw your transfer request?" action="withdraw_request">
                              <p>You stay. The manager and the fans will remember you asked.</p>
                            </ConfirmAction>
                          </div>
                        </>
                      ) : (
                        <>
                          <p className="muted">Asking to leave is not private for long. Clubs may respond, supporters and the press will notice, and the manager will remember.</p>
                          <div className="formfoot">
                            <ConfirmAction label="Hand in a transfer request" title="Hand in a transfer request?" action="transfer_request" danger>
                              <p>The club is told the next day. It may put you up for sale, refuse, or ignore it, and people will talk. You can withdraw the request, but not the fact that you made it.</p>
                            </ConfirmAction>
                          </div>
                        </>
                      )}
                    </div>
                  </Section>
                  <Section title="Your agent">
                    <AgentPanel today={today} />
                  </Section>
                  <Section title="Offers waiting">
                    <div className="card list-card">
                      {c.offers && c.offers.length > 0 ? (
                        <ul className="rows">
                          {c.offers.map((o) => (
                            <li key={o.id}>
                              <span>{o.title}</span>
                              <a className="btn btn-primary btn-sm" href={href(`/messages/${o.id}`)}>Open</a>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <div className="muted pad">No offers are waiting. Clubs approach you as time passes.</div>
                      )}
                    </div>
                    <p className="hint">Offers can be accepted or declined. Counter-proposals are not available yet.</p>
                  </Section>
                  {c.history && c.history.length > 0 && (
                    <Section title="Past contracts">
                      <div className="card list-card">
                        <ul className="rows">
                          {c.history.map((h, i) => (
                            <li key={i}><span><Parts parts={h.parts} /></span><span className="hint"><Dt d={h.date} /></span></li>
                          ))}
                        </ul>
                      </div>
                    </Section>
                  )}
                </aside>
              </div>
            </>
          )
        }
      </Async>
    </div>
  );
}
