export function HomeMark({ className = 'header-home-mark' }: { className?: string }) {
  return (
    <svg className={className} viewBox="8 8 16 16" aria-hidden="true" focusable="false">
      <path d="M8 8h7v9h9v7H8z" fill="#1d1d1f" />
      <rect x="18" y="8" width="6" height="6" rx="1.5" fill="#0071e3" />
    </svg>
  );
}

export function PageBackHome() {
  return (
    <a href="#/home" className="page-back-home" aria-label="Home">
      <span className="page-back-home-chevron" aria-hidden="true">
        ←
      </span>
    </a>
  );
}
