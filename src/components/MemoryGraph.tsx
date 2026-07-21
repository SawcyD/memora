import { useMemo } from "react";
import type { MemorySnapshot } from "@/system/types";

/** Leading-edge axis label, singular where the count is one. */
function axisLabel(seconds: number): string {
  if (seconds < 60) return `${seconds} seconds`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return minutes === 1 ? "1 minute" : `${minutes} minutes`;
  const hours = Math.round(minutes / 60);
  return hours === 1 ? "1 hour" : `${hours} hours`;
}

/**
 * Task Manager's memory graph: a filled area against a fixed 0–100% axis with a
 * faint grid. No gradients, no animation, no decoration — the shape is the
 * information.
 */
export function MemoryGraph({
  history,
  seconds,
  className,
}: {
  history: MemorySnapshot[];
  /** Width of the visible window, in seconds. */
  seconds: number;
  className?: string;
}) {
  const W = 1000;
  const H = 260;

  const path = useMemo(() => {
    const pts = history.slice(-seconds);
    if (pts.length < 2) return null;

    // The graph is right-anchored: the newest sample sits at x = W, and a
    // partially filled buffer leaves the left side empty rather than stretching.
    const step = W / (seconds - 1);
    const x = (i: number) => W - (pts.length - 1 - i) * step;
    const y = (p: MemorySnapshot) => H - (p.percentInUse / 100) * H;

    const line = pts.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p).toFixed(1)}`);
    return {
      line: line.join(" "),
      area: `${line.join(" ")} L${W},${H} L${x(0).toFixed(1)},${H} Z`,
    };
  }, [history, seconds]);

  return (
    <div
      className={[
        "relative rounded-[var(--radius-md)] border border-[var(--stroke-divider)]",
        "bg-[var(--card-fill-secondary)]",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <svg
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        className="block h-[180px] w-full"
        role="img"
        aria-label={
          history.length
            ? `Memory usage over the last ${seconds} seconds. Currently ${Math.round(
                history[history.length - 1].percentInUse,
              )} percent in use.`
            : "Memory usage graph, waiting for data."
        }
      >
        {/* Task Manager draws a 10 x 6 grid behind the plot. */}
        <g stroke="var(--stroke-divider)" strokeWidth="1" shapeRendering="crispEdges">
          {Array.from({ length: 9 }, (_, i) => (
            <line key={`v${i}`} x1={((i + 1) * W) / 10} y1={0} x2={((i + 1) * W) / 10} y2={H} />
          ))}
          {Array.from({ length: 5 }, (_, i) => (
            <line key={`h${i}`} x1={0} y1={((i + 1) * H) / 6} x2={W} y2={((i + 1) * H) / 6} />
          ))}
        </g>

        {path && (
          <>
            <path d={path.area} fill="var(--accent-usable)" opacity="0.22" />
            <path
              d={path.line}
              fill="none"
              stroke="var(--accent-usable)"
              strokeWidth="1.5"
              vectorEffect="non-scaling-stroke"
            />
          </>
        )}
      </svg>

      <div className="flex justify-between px-2 pb-1 text-[11px] text-[var(--text-tertiary)]">
        <span>{axisLabel(seconds)}</span>
        <span>0</span>
      </div>
      <span className="absolute right-2 top-1 text-[11px] text-[var(--text-tertiary)]">
        100%
      </span>
    </div>
  );
}
