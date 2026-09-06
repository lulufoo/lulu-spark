import { state } from '../../state/host.ts';
import * as api from '../../../host/api.ts';

const kbBlobUrls = new Set<string>();

export function revokeKbBlobUrls() {
  for (const url of kbBlobUrls) {
    URL.revokeObjectURL(url);
  }
  kbBlobUrls.clear();
}

function isExternalOrSpecialImgSrc(src: string) {
  return /^(https?:|data:|blob:|\/)/i.test(src);
}

export async function hydrateKbRelativeImages(container: Element) {
  const repo = state.viewer.kbRepo;
  const base = state.viewer.kbPath;
  if (!repo || !base) return;
  const imgs = [...container.querySelectorAll('img[src]')];
  revokeKbBlobUrls();
  await Promise.all(
    imgs.map(async (img) => {
      const href = img.getAttribute('src');
      if (!href || href.startsWith('#') || isExternalOrSpecialImgSrc(href)) return;
      try {
        const blobUrl = (await api.fetchKbAssetAsBlobUrl(repo, base, href)) as string;
        kbBlobUrls.add(blobUrl);
        (img as HTMLImageElement).src = blobUrl;
      } catch {
        if (!(img as HTMLImageElement).alt) (img as HTMLImageElement).alt = href;
      }
    }),
  );
}
