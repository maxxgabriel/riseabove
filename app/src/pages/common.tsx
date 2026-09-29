import { useEffect, type ReactNode } from "react";
import { EntityLink } from "../components/links";
import { href } from "../router";
import { useApi } from "../store";
import type { Ref } from "../types";
import { ErrorState, Skeleton } from "../ui/ui";

export function usePageTitle(title: string | undefined) {
  useEffect(() => {
    document.title = title ? `${title} · Rise Above` : "Rise Above";
  }, [title]);
}

export interface Crumb {
  label: string;
  to?: string;
  r?: Ref;
}

export function Crumbs({ items }: { items: Crumb[] }) {
  return (
    <nav className="crumbs" aria-label="Breadcrumb">
      {items.map((c, i) => (
        <span key={i} className="crumb">
          {i > 0 && <span aria-hidden="true"> / </span>}
          {c.r ? <EntityLink r={c.r}>{c.label}</EntityLink> : c.to ? <a href={href(c.to)}>{c.label}</a> : <span>{c.label}</span>}
        </span>
      ))}
    </nav>
  );
}

export function PageHead({ title, sub, actions, crumbs }: { title: ReactNode; sub?: ReactNode; actions?: ReactNode; crumbs?: Crumb[] }) {
  return (
    <div className="page-head">
      <div>
        {crumbs && <Crumbs items={crumbs} />}
        <h1>{title}</h1>
        {sub && <div className="page-sub">{sub}</div>}
      </div>
      {actions && <div className="page-actions">{actions}</div>}
    </div>
  );
}

/** Render the result of an API call with a loading skeleton and a readable error. */
export function Async<T>({ q, children, skeleton }: { q: ReturnType<typeof useApi<T>>; children: (d: T) => ReactNode; skeleton?: ReactNode }) {
  if (q.error && !q.data) return <ErrorState error={q.error} onRetry={q.reload} />;
  if (!q.data) {
    return (
      skeleton ?? (
        <div className="page-skel" aria-busy="true">
          <Skeleton w="30%" h={22} />
          <Skeleton w="60%" />
          <Skeleton w="50%" />
          <Skeleton w="70%" />
        </div>
      )
    );
  }
  return <>{children(q.data)}</>;
}
