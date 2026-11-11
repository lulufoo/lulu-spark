import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

type WindowState = {
  isFullscreen: () => Promise<boolean>;
  listen: (event: string, callback: (event: { payload: boolean }) => void) => Promise<() => void>;
};

/** Follow native fullscreen, not browser fullscreen or ordinary maximization. */
export function watchNativeFullscreen(windowState: WindowState, update: (value: boolean) => void): () => void {
  let disposed = false;
  let sequence = 0;
  let unlisten: (() => void) | undefined;

  const refresh = () => {
    const current = ++sequence;
    void windowState.isFullscreen().then((fullscreen) => {
      if (!disposed && current === sequence) update(fullscreen);
    }).catch(() => {
      // Preserve clearance for the window controls if native state is unavailable.
      if (!disposed && current === sequence) update(false);
    });
  };
  const onFullscreenChanged = (event: { payload: boolean }) => {
    if (disposed) return;
    sequence++;
    update(event.payload);
  };
  void windowState.listen('spark-native-fullscreen-changed', onFullscreenChanged).then((stop) => {
    if (disposed) stop();
    else {
      unlisten = stop;
      refresh();
    }
  }).catch(() => {
    if (!disposed) refresh();
  });

  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function useNativeMacFullscreen(): boolean {
  const [fullscreen, setFullscreen] = useState(false);
  useEffect(() => {
    if (!isTauri() || !/Mac/i.test(navigator.platform)) return;
    return watchNativeFullscreen(getCurrentWindow(), setFullscreen);
  }, []);
  return fullscreen;
}
