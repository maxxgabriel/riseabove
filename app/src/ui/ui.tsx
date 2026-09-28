import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ComponentPropsWithRef,
  type ButtonHTMLAttributes,
  type KeyboardEvent as RKeyboardEvent,
  type ReactNode,
} from "react";
import { Icon, type IconName } from "./Icon";
import { Popover } from "./Popover";

// ---- buttons -------------------------------------------------------------------------------

interface ButtonProps extends ComponentPropsWithRef<"button"> {
  variant?: "default" | "primary" | "ghost" | "danger";
  icon?: IconName;
  size?: "sm" | "md";
}

export function Button({ variant = "default", icon, size = "md", className = "", children, type = "button", ...rest }: ButtonProps) {
  return (
    <button type={type} className={`btn btn-${variant} btn-${size} ${className}`} {...rest}>
      {icon && <Icon name={icon} size={size === "sm" ? 14 : 16} />}
      {children}
    </button>
  );
}

export function IconButton({ icon, label, className = "", size = 16, ...rest }: { icon: IconName; label: string; size?: number } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button type="button" className={`iconbtn ${className}`} aria-label={label} title={label} {...rest}>
      <Icon name={icon} size={size} />
    </button>
  );
}

// ---- menu ----------------------------------------------------------------------------------

export type MenuItem =
  | { kind?: "item"; label: string; hint?: string; icon?: IconName; onSelect: () => void; disabled?: boolean; checked?: boolean; danger?: boolean }
  | { kind: "sep" }
  | { kind: "label"; label: string };

interface MenuProps {
  items: MenuItem[];
  align?: "start" | "end";
  label?: string;
  children: (t: { setRef: (el: HTMLElement | null) => void; open: boolean; toggle: () => void }) => ReactNode;
}

export function Menu({ items, align = "start", label, children }: MenuProps) {
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (open) {
      const first = listRef.current?.querySelector<HTMLElement>('[role="menuitem"]:not([disabled])');
      first?.focus();
    }
  }, [open]);

  const onKey = (e: RKeyboardEvent) => {
    const els = Array.from(listRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]:not([disabled])') ?? []);
    const i = els.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      els[(i + 1) % els.length]?.focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      els[(i - 1 + els.length) % els.length]?.focus();
    } else if (e.key === "Home") {
      e.preventDefault();
      els[0]?.focus();
    } else if (e.key === "End") {
      e.preventDefault();
      els[els.length - 1]?.focus();
    } else if (e.key === "Tab") {
      close();
    }
  };

  return (
    <>
      {children({ setRef: setAnchor, open, toggle: () => setOpen((o) => !o) })}
      <Popover anchor={anchor} open={open} onClose={close} align={align} className="menu" label={label}>
        <div ref={listRef} role="menu" aria-label={label} onKeyDown={onKey}>
          {items.map((it, i) => {
            if (it.kind === "sep") return <div key={i} className="menu-sep" role="separator" />;
            if (it.kind === "label") return <div key={i} className="menu-label">{it.label}</div>;
            return (
              <button
                key={i}
                type="button"
                role={it.checked != null ? "menuitemcheckbox" : "menuitem"}
                aria-checked={it.checked}
                disabled={it.disabled}
                className={`menu-item ${it.danger ? "danger" : ""}`}
                onClick={() => {
                  close();
                  anchor?.focus();
                  it.onSelect();
                }}
              >
                <span className="menu-check">{it.checked ? <Icon name="check" size={14} /> : it.icon ? <Icon name={it.icon} size={14} /> : null}</span>
                <span className="menu-text">{it.label}</span>
                {it.hint && <span className="menu-hint">{it.hint}</span>}
              </button>
            );
          })}
        </div>
      </Popover>
    </>
  );
}

// ---- dialog --------------------------------------------------------------------------------

export function Dialog({
  open,
  onClose,
  title,
  children,
  footer,
  width = 460,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const id = useId();
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);
  return (
    <dialog
      ref={ref}
      className="dialog"
      aria-labelledby={id}
      style={{ width }}
      onClose={onClose}
      onMouseDown={(e) => {
        if (e.target === ref.current) onClose();
      }}
    >
      {open && (
        <div className="dialog-body">
          <header className="dialog-head">
            <h2 id={id}>{title}</h2>
            <IconButton icon="x" label="Close" onClick={onClose} />
          </header>
          <div className="dialog-content">{children}</div>
          {footer && <footer className="dialog-foot">{footer}</footer>}
        </div>
      )}
    </dialog>
  );
}

// ---- selection controls ----------------------------------------------------------------------

export function Tabs<T extends string>({ tabs, value, onChange, label }: { tabs: { id: T; label: string; count?: number; hidden?: boolean }[]; value: T; onChange: (id: T) => void; label?: string }) {
  const shown = tabs.filter((t) => !t.hidden);
  const onKey = (e: RKeyboardEvent) => {
    const i = shown.findIndex((t) => t.id === value);
    if (e.key === "ArrowRight") onChange(shown[(i + 1) % shown.length].id);
    else if (e.key === "ArrowLeft") onChange(shown[(i - 1 + shown.length) % shown.length].id);
    else return;
    e.preventDefault();
  };
  return (
    <div className="tabs" role="tablist" aria-label={label} onKeyDown={onKey}>
      {shown.map((t) => (
        <button key={t.id} role="tab" type="button" aria-selected={t.id === value} tabIndex={t.id === value ? 0 : -1} className="tab" onClick={() => onChange(t.id)}>
          {t.label}
          {t.count != null && <span className="tab-count">{t.count}</span>}
        </button>
      ))}
    </div>
  );
}

export function Segmented<T extends string | number>({ options, value, onChange, label, size = "md" }: { options: { id: T; label: string; title?: string }[]; value: T; onChange: (v: T) => void; label: string; size?: "sm" | "md" }) {
  return (
    <div className={`seg seg-${size}`} role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button key={String(o.id)} type="button" role="radio" aria-checked={o.id === value} title={o.title} className="seg-opt" onClick={() => onChange(o.id)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Switch({ checked, onChange, label, hint }: { checked: boolean; onChange: (v: boolean) => void; label: string; hint?: string }) {
  const id = useId();
  return (
    <label className="switch" htmlFor={id}>
      <span className="switch-text">
        <span>{label}</span>
        {hint && <span className="hint">{hint}</span>}
      </span>
      <input id={id} type="checkbox" role="switch" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span className="switch-track" aria-hidden="true">
        <span className="switch-thumb" />
      </span>
    </label>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {hint && <span className="hint">{hint}</span>}
    </label>
  );
}

// ---- small display pieces -------------------------------------------------------------------

export function Badge({ tone, children, title }: { tone?: "pos" | "neg" | "warn" | "info" | "muted" | "you"; children: ReactNode; title?: string }) {
  return (
    <span className={`badge ${tone ? `badge-${tone}` : ""}`} title={title}>
      {children}
    </span>
  );
}

export function Spinner({ size = 16, label = "Loading" }: { size?: number; label?: string }) {
  return <span className="spinner" style={{ width: size, height: size }} role="status" aria-label={label} />;
}

export function Skeleton({ w, h = 14 }: { w?: number | string; h?: number }) {
  return <span className="skeleton" style={{ width: w ?? "100%", height: h }} />;
}

export function Empty({ title, children, icon = "info" }: { title: string; children?: ReactNode; icon?: IconName }) {
  return (
    <div className="empty">
      <Icon name={icon} size={20} />
      <div className="empty-title">{title}</div>
      {children && <div className="empty-body">{children}</div>}
    </div>
  );
}

export function ErrorState({ error, onRetry }: { error: { message: string; code?: string }; onRetry?: () => void }) {
  return (
    <div className="errorstate" role="alert">
      <Icon name="warn" size={18} />
      <div>
        <div className="empty-title">Something went wrong</div>
        <div className="empty-body">{error.message}</div>
      </div>
      {onRetry && (
        <Button size="sm" onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}

export function Section({ title, aside, children, className = "", id }: { title?: ReactNode; aside?: ReactNode; children: ReactNode; className?: string; id?: string }) {
  return (
    <section className={`section ${className}`} id={id}>
      {(title || aside) && (
        <header className="section-head">
          {title && <h2>{title}</h2>}
          {aside && <div className="section-aside">{aside}</div>}
        </header>
      )}
      {children}
    </section>
  );
}

export function KeyVal({ rows, className = "" }: { rows: { k: string; v: ReactNode; hint?: string }[]; className?: string }) {
  return (
    <dl className={`kv ${className}`}>
      {rows.map((r, i) => (
        <div key={i} className="kv-row">
          <dt title={r.hint}>{r.k}</dt>
          <dd>{r.v}</dd>
        </div>
      ))}
    </dl>
  );
}

/** A 0-100 value with the word the game uses for it. */
export function Meter({ value, label, tone }: { value: number; label?: string; tone?: "pos" | "neg" | "warn" }) {
  const t = tone ?? (value >= 66 ? "pos" : value >= 40 ? "warn" : "neg");
  return (
    <span className="meter" title={`${Math.round(value)} / 100`}>
      <span className="meter-track">
        <span className={`meter-fill tone-${t}`} style={{ width: `${Math.max(2, Math.min(100, value))}%` }} />
      </span>
      {label && <span className="meter-label">{label}</span>}
    </span>
  );
}

export function Avatar({ initials, size = 40, you = false }: { initials: string; size?: number; you?: boolean }) {
  return (
    <span className={`avatar ${you ? "avatar-you" : ""}`} style={{ width: size, height: size, fontSize: size * 0.36 }} aria-hidden="true">
      {initials}
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="kbd">{children}</kbd>;
}

export function Progress({ value, max = 100, label }: { value: number; max?: number; label?: string }) {
  return (
    <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={max} aria-valuenow={value} aria-label={label}>
      <div className="progress-fill" style={{ width: `${max ? Math.min(100, (value / max) * 100) : 0}%` }} />
    </div>
  );
}
