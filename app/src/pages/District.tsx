import type { RouteOptionsView } from "../contract.generated";
import { useMemo } from "react";
import { EntityLink } from "../components/links";
import type { DistrictView } from "../contract.generated";
import { setQuery, useRoute } from "../router";
import { useApi } from "../store";
import { Badge, Empty, Field, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type Options = RouteOptionsView;

const TONE: Record<string, "pos" | "neg" | "warn" | "muted"> = { "very low": "neg", low: "warn", middling: "muted", high: "pos", "very high": "pos" };

/** What a place is like for a child growing up in it, and who is near enough to notice them. Defaults to the home district of the person lived as. */
export function District() {
  usePageTitle("Your district");
  const route = useRoute();
  const raw = route.query.get("id");
  const id = raw != null && raw !== "" && Number.isFinite(Number(raw)) ? Number(raw) : null;
  const opts = useApi<Options>("route.options");
  const q = useApi<DistrictView>("ecosystem.district", id == null ? {} : { id });
  const all = useMemo(() => (opts.data?.states ?? []).flatMap((s) => s.districts.map((d) => ({ id: d.id, label: `${d.name}, ${s.name}` }))), [opts.data]);
  return (
    <div className="page narrow">
      <PageHead title="Your district" sub="Where you grow up decides how easily you are seen. This is what the place is like, not how good you are." />
      {all.length > 0 && (
        <Field label="Look at another district">
          <select value={id ?? ""} onChange={(e) => setQuery({ id: e.target.value === "" ? null : e.target.value })}>
            <option value="">{id == null ? "Your own" : "Back to your own"}</option>
            {all.map((d) => (
              <option key={d.id} value={d.id}>
                {d.label}
              </option>
            ))}
          </select>
        </Field>
      )}
      <Async q={q}>
        {(d) =>
          !d.available ? (
            <Empty title="No district to show">{d.reason}</Empty>
          ) : (
            <>
              <Section title={d.name} aside={`${d.state} · ${d.population_k.toLocaleString()} thousand people`}>
                {d.association && <p className="muted">{d.association}</p>}
                <div className="card">
                  <ul className="rows">
                    {d.aspects.map((a) => (
                      <li key={a.label}>
                        <div className="row between">
                          <strong>{a.label}</strong>
                          <Badge tone={TONE[a.level] ?? "muted"}>{a.level}</Badge>
                        </div>
                        <div className="muted">{a.note}</div>
                      </li>
                    ))}
                  </ul>
                </div>
              </Section>
              <Section title="Who is near enough to notice you" aside="Within reasonable travel of this district">
                <div className="card">
                  <p>
                    <strong>Academies</strong>
                  </p>
                  {d.academies.length === 0 ? (
                    <p className="muted">No professional academy is within reach. Getting seen means going to them: state sides, camps and trials.</p>
                  ) : (
                    <ul className="plain">
                      {d.academies.map((a) => (
                        <li key={a.id}>
                          <EntityLink r={a}>{a.name}</EntityLink>
                        </li>
                      ))}
                    </ul>
                  )}
                  <p>
                    <strong>Universities</strong>
                  </p>
                  {d.universities.length === 0 ? <p className="muted">No university is within reach.</p> : <ul className="plain">{d.universities.map((u) => <li key={u}>{u}</li>)}</ul>}
                  <p className="muted">{d.schools} {d.schools === 1 ? "school plays" : "schools play"} organised football here.</p>
                </div>
              </Section>
            </>
          )
        }
      </Async>
    </div>
  );
}
