const RING_SIZE = 15;
const RING_STROKE = 2;
const RING_R = 5.5;
const RING_C = 2 * Math.PI * RING_R;

export function ContextPercent({ percent }: { percent: number | null }) {
  const shown = Math.min(100, Math.max(percent ?? 1, 1));
  const offset = RING_C - (shown / 100) * RING_C;
  const label = `${shown}%`;
  return (
    <span className="home-chat-context-percent" data-role="context-percent" title={label} aria-label={label}>
      <svg
        className="home-chat-context-ring"
        width={RING_SIZE}
        height={RING_SIZE}
        viewBox={`0 0 ${RING_SIZE} ${RING_SIZE}`}
        aria-hidden="true"
      >
        <circle
          className="home-chat-context-ring-track"
          cx={RING_SIZE / 2}
          cy={RING_SIZE / 2}
          r={RING_R}
          fill="none"
          strokeWidth={RING_STROKE}
        />
        <circle
          className="home-chat-context-ring-progress"
          cx={RING_SIZE / 2}
          cy={RING_SIZE / 2}
          r={RING_R}
          fill="none"
          strokeWidth={RING_STROKE}
          strokeLinecap="round"
          strokeDasharray={RING_C}
          strokeDashoffset={offset}
          transform={`rotate(-90 ${RING_SIZE / 2} ${RING_SIZE / 2})`}
        />
      </svg>
    </span>
  );
}
