import { useId } from "react";
import { brandFor, hashOf, initials, readableOn } from "../color";
import { useClubColors } from "../crest";

interface CrestProps {
  /** Club, nation or competition name; the initials come from it. */
  name: string;
  /** Primary and secondary colours as hex. Without them a stable pair is made from the id. */
  colors?: [string, string] | string[] | null;
  id?: number;
  kind?: "club" | "comp" | "nation" | "person";
  size?: number;
  className?: string;
  /** Force plain initials off (used at tiny sizes automatically). */
  plain?: boolean;
}

const SHIELD = "M3 4.2C9 2.2 15 2 16 1c1 1 7 1.2 13 3.2V19c0 7.4-6.6 12.4-13 15C9.6 31.4 3 26.4 3 19Z";

/** A small generated badge: a shield in the club's colours with a pattern picked from its id. Not a real crest. */
export function Crest({ name, colors, id = 0, kind = "club", size = 24, className = "", plain }: CrestProps) {
  const uid = useId().replace(/:/g, "");
  const [c1, c2] = colors && colors.length >= 2 ? [colors[0], colors[1]] : brandFor(kind === "person" ? "nation" : kind, id, name);
  const pattern = hashOf(`${kind}${id}`) % 5;
  const ink = readableOn(c1);
  const showText = !plain && size >= 26;
  const round = kind === "nation";
  return (
    <svg className={`crest ${className}`} viewBox="0 0 32 36" width={size} height={size * 1.125} role="img" aria-label={`${name} badge`}>
      <defs>
        <clipPath id={`c${uid}`}>{round ? <circle cx="16" cy="18" r="15" /> : <path d={SHIELD} />}</clipPath>
      </defs>
      <g clipPath={`url(#c${uid})`}>
        <rect width="32" height="36" fill={c1} />
        {pattern === 0 && <rect x="16" width="16" height="36" fill={c2} />}
        {pattern === 1 && <rect y="13" width="32" height="9" fill={c2} />}
        {pattern === 2 && <path d="M-2 10 16 22 34 10V17L16 29-2 17Z" fill={c2} />}
        {pattern === 3 && (
          <>
            <rect x="10" width="5" height="36" fill={c2} />
            <rect x="21" width="5" height="36" fill={c2} />
          </>
        )}
        {pattern === 4 && <circle cx="16" cy="17" r="9" fill={c2} />}
      </g>
      {round ? <circle cx="16" cy="18" r="14.6" fill="none" stroke="rgba(255,255,255,0.55)" strokeWidth="1.4" /> : <path d={SHIELD} fill="none" stroke="rgba(255,255,255,0.55)" strokeWidth="1.4" />}
      {showText && (
        <text x="16" y="21.5" textAnchor="middle" fontSize="11" fontWeight="800" fontFamily="'Barlow Condensed', var(--font)" fill={pattern === 4 ? readableOn(c2) : ink} stroke={pattern === 4 ? "none" : c1} strokeWidth="0.4" paintOrder="stroke">
          {initials(name)}
        </text>
      )}
    </svg>
  );
}

/** A club's badge with its own colours, looked up by id. */
export function ClubCrest({ id, name, size = 22, plain = true }: { id: number; name: string; size?: number; plain?: boolean }) {
  const colors = useClubColors(id);
  return <Crest name={name} colors={colors} id={id} size={size} plain={plain} />;
}
