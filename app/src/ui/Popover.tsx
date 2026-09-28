import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

interface PopoverProps {
  anchor: HTMLElement | null;
  open: boolean;
  onClose: () => void;
  align?: "start" | "end";
  children: ReactNode;
  className?: string;
  minWidth?: number | "anchor";
  label?: string;
}

/** A floating panel anchored to an element. Closes on outside click and Escape. */
export function Popover({ anchor, open, onClose, align = "start", children, className = "", minWidth, label }: PopoverProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number; maxH: number } | null>(null);

  const place = useCallback(() => {
    if (!anchor || !ref.current) return;
    const a = anchor.getBoundingClientRect();
    const p = ref.current.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let left = align === "end" ? a.right - p.width : a.left;
    left = Math.max(8, Math.min(left, vw - p.width - 8));
    const below = vh - a.bottom - 8;
    const above = a.top - 8;
    let top = a.bottom + 4;
    let maxH = below - 4;
    if (p.height > below && above > below) {
      top = Math.max(8, a.top - 4 - Math.min(p.height, above));
      maxH = above - 4;
    }
    setPos({ top, left, maxH });
  }, [anchor, align]);

  useLayoutEffect(() => {
    if (open) place();
    else setPos(null);
  }, [open, place, children]);

  useEffect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      const t = e.target as Node;
      if (ref.current?.contains(t) || anchor?.contains(t)) return;
      onClose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
        anchor?.focus();
      }
    };
    const resize = () => place();
    document.addEventListener("mousedown", down, true);
    document.addEventListener("keydown", key, true);
    window.addEventListener("resize", resize);
    return () => {
      document.removeEventListener("mousedown", down, true);
      document.removeEventListener("keydown", key, true);
      window.removeEventListener("resize", resize);
    };
  }, [open, anchor, onClose, place]);

  if (!open) return null;
  const width = minWidth === "anchor" ? anchor?.getBoundingClientRect().width : minWidth;
  return createPortal(
    <div
      ref={ref}
      className={`popover ${className}`}
      role="dialog"
      aria-label={label}
      style={{ top: pos?.top ?? -9999, left: pos?.left ?? -9999, maxHeight: pos?.maxH, minWidth: width, visibility: pos ? "visible" : "hidden" }}
    >
      {children}
    </div>,
    document.body,
  );
}
