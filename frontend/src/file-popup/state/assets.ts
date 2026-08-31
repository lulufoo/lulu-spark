const assetBlobUrls = new Set<string>();

export function rememberFilePopupAssetUrl(url: string) {
  assetBlobUrls.add(url);
}

export function revokeFilePopupAssetUrls() {
  for (const url of assetBlobUrls) {
    URL.revokeObjectURL(url);
  }
  assetBlobUrls.clear();
}
