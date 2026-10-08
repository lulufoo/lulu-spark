import { useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import { closeAbout } from '../commands/about.ts';
import { aboutStore } from '../state/about.ts';

export function AboutDialog() {
  const about = useSyncExternalStore(aboutStore.subscribe, aboutStore.getSnapshot);

  return (
    <div
      id="about-dialog"
      className={about.open ? 'open' : undefined}
      data-role="about-dialog"
      data-open={about.open ? 'true' : 'false'}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeAbout();
      }}
    >
      <div id="about-dialog-box" role="dialog" aria-modal="true" aria-labelledby="about-dialog-title">
        <div id="about-dialog-header">
          <h3 id="about-dialog-title">About</h3>
          <OverlayDismissButton id="btn-about-close" title="Close" onClick={() => closeAbout()} />
        </div>
        <p data-role="about-product-name">{about.product_name}</p>
        <p data-role="about-version">{about.version}</p>
      </div>
    </div>
  );
}
