import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { version } from 'react';
import { App } from './App.tsx';
import './markdown.ts';

const host = document.getElementById('root');
if (!host) {
  throw new Error('missing #root');
}
flushSync(() => {
  createRoot(host).render(<App />);
});
window.__WB_P0_PIPELINE__ = { react: version };

void import('./boot.ts');
