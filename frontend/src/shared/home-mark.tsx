export function HomeMark({ className = 'header-home-mark' }: { className?: string }) {
  return (
    <svg className={className} viewBox="8 8 16 16" aria-hidden="true" focusable="false">
      <path d="M8 8h7v9h9v7H8z" fill="#576AE0" />
      <rect x="18" y="8" width="6" height="6" rx="1.5" fill="#FF5A4E" />
    </svg>
  );
}

export function PageBackHome() {
  return (
    <a href="#/home" className="page-back-home" aria-label="Home">
      <svg
        className="page-back-home-icon"
        viewBox="0 0 24 24"
        aria-hidden="true"
        focusable="false"
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="m12 19-7-7 7-7" />
        <path d="M19 12H5" />
      </svg>
    </a>
  );
}
