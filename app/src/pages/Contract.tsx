import { EntityLink, Money, Dt } from "../components/links";
import { duration } from "../format";
import { href } from "../router";
import { useApi } from "../store";
import type { Named, Part } from "../types";
import { Parts } from "../components/links";
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
}

export function Contract() {
  usePageTitle("Contract");
  const q = useApi<ContractResp>("me.contract");
  return (
    <div className="page">
      <Async q={q}>
        {(c) =>
          !c.has_contract ? (
            <>
              <PageHead title="Contract" />
              <Empty title="You do not have a contract" icon="contract">You are {c.status?.toLowerCase() ?? "without a club"}. Offers from clubs arrive in your <a href={href("/messages")}>messages</a>.</Empty>
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
