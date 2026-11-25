// One measurement over time: a line with a light wash under it.
//
// A single series in a single hue, so it needs no legend; the panel's title
// says what is drawn. Moving the pointer anywhere over the plot reads off
// the nearest point, so nobody has to aim at a two-pixel line.

import { useEffect, useRef, useState } from 'preact/hooks';
import { clock, when } from '../util.ts';

export interface Point {
  at: number;
  value: number;
}

const HEIGHT = 132;
const PAD = { top: 10, right: 12, bottom: 22 };

export function Trend(props: { points: Point[]; since: number; until: number; format: (value: number) => string; label: string }) {
  const { points, since, until, format } = props;
  const host = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const box = host.current;
    if (!box) return;
    const observer = new ResizeObserver(() => setWidth(box.clientWidth));
    observer.observe(box);
    setWidth(box.clientWidth);
    return () => observer.disconnect();
  }, []);

  if (points.length === 0) {
    return (
      <div ref={host} class="empty" style={`height:${HEIGHT}px;padding:0`}>
        Nothing was measured in this window.
      </div>
    );
  }

  const top = niceCeil(Math.max(...points.map((point) => point.value)));
  // Room on the left for the widest tick label, so none is cut off.
  const left = 16 + Math.max(...[0, 0.5, 1].map((share) => format(top * share).length)) * 6.4;
  const plotW = Math.max(10, width - left - PAD.right);
  const plotH = HEIGHT - PAD.top - PAD.bottom;
  const x = (at: number) => left + ((at - since) / Math.max(1, until - since)) * plotW;
  const y = (value: number) => PAD.top + plotH - (value / top) * plotH;
  const line = points.map((point, index) => `${index ? 'L' : 'M'}${x(point.at).toFixed(1)},${y(point.value).toFixed(1)}`).join(' ');
  const area = `${line} L${x(points.at(-1)!.at).toFixed(1)},${PAD.top + plotH} L${x(points[0]!.at).toFixed(1)},${PAD.top + plotH} Z`;
  const shown = hover === null ? null : points[hover]!;
  const last = points.at(-1)!;
  const long = until - since > 86_400_000;

  return (
    <div
      ref={host}
      class="trend"
      role="img"
      aria-label={`${props.label}: latest ${format(last.value)}, highest ${format(Math.max(...points.map((p) => p.value)))}, over ${points.length} points`}
      onPointerMove={(event) => {
        const left = event.clientX - event.currentTarget.getBoundingClientRect().left;
        let best = 0;
        for (let i = 1; i < points.length; i++) {
          if (Math.abs(x(points[i]!.at) - left) < Math.abs(x(points[best]!.at) - left)) best = i;
        }
        setHover(best);
      }}
      onPointerLeave={() => setHover(null)}
    >
      {width > 0 && (
        <svg width={width} height={HEIGHT}>
          {[0, 0.5, 1].map((share) => (
            <g key={share}>
              <line class="grid" x1={left} x2={left + plotW} y1={y(top * share)} y2={y(top * share)} />
              <text class="tick" x={left - 8} y={y(top * share) + 4} text-anchor="end">
                {format(top * share)}
              </text>
            </g>
          ))}
          <text class="tick" x={left} y={HEIGHT - 5}>
            {long ? when(since) : clock(since)}
          </text>
          <text class="tick" x={left + plotW} y={HEIGHT - 5} text-anchor="end">
            now
          </text>
          <path class="wash" d={area} />
          <path class="line" d={line} />
          {points.length === 1 && <circle class="dot" cx={x(last.at)} cy={y(last.value)} r="4" />}
          {shown && (
            <>
              <line class="cross" x1={x(shown.at)} x2={x(shown.at)} y1={PAD.top} y2={PAD.top + plotH} />
              <circle class="dot" cx={x(shown.at)} cy={y(shown.value)} r="4" />
            </>
          )}
        </svg>
      )}
      {shown && (
        <div class="tip" style={`left:${Math.min(Math.max(x(shown.at), 70), width - 70)}px;top:${Math.max(0, y(shown.value) - 48)}px`}>
          <strong>{format(shown.value)}</strong>
          <span>{long ? when(shown.at) : clock(shown.at)}</span>
        </div>
      )}
    </div>
  );
}

/** The next "round" number at or above `value`, for the top of an axis. */
export function niceCeil(value: number): number {
  if (!(value > 0)) return 1;
  const power = 10 ** Math.floor(Math.log10(value));
  for (const step of [1, 2, 2.5, 5, 10]) {
    if (step * power >= value) return step * power;
  }
  return 10 * power;
}
