const PROMPTS = [
  { label: 'Find documents', text: 'Find documents about ' },
  { label: 'Overview', text: 'Summarize my workspaces' },
  { label: 'Read Later', text: 'What did I save to Read Later this week?' },
];

/** Empty state of the chat pane: a greeting and a few starter prompts that fill the composer. */
export function ChatWelcome({ onPick }: { onPick: (text: string) => void }) {
  return (
    <div className="home-welcome">
      {/* The app icon (icons/app-icon.svg), same geometry and colours. */}
      <svg className="home-welcome-mark" viewBox="0 0 32 32" aria-hidden="true" focusable="false">
        <rect x="2" y="2" width="28" height="28" rx="8" fill="#1d1d1f" />
        <path d="M8 8h7v9h9v7H8z" fill="#f5f5f7" />
        <rect x="18" y="8" width="6" height="6" rx="1.5" fill="#0071e3" />
      </svg>
      <h2 className="home-welcome-title">What can I help with?</h2>
      <div className="home-welcome-prompts">
        {PROMPTS.map((p) => (
          <button
            key={p.label}
            type="button"
            className="home-welcome-prompt"
            onClick={() => onPick(p.text)}
          >
            <span className="home-welcome-prompt-label">{p.label}</span>
            <span className="home-welcome-prompt-text">{p.text}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
