import * as api from '../../host/api.ts';

export type ResultTone = '' | 'ok' | 'warn' | 'err' | 'busy';

export async function doMoveDoc(
  srcUrl: string,
  dstUrl: string,
  setResult: (tone: ResultTone, text: string) => void,
) {
  if (!srcUrl || !dstUrl) {
    setResult('err', 'Enter both URLs');
    return;
  }
  setResult('busy', 'Running gh api…');
  try {
    const data = await api.ghMove(srcUrl, dstUrl);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      const movedInfo = data.moved !== undefined ? ` (${data.moved} files moved)` : '';
      setResult('warn', `⚠ ${data.warn}${movedInfo}`);
      return;
    }
    const movedInfo = data.moved !== undefined ? ` (${data.moved} files total)` : '';
    setResult('ok', `✓ Moved to ${data.dst_path}${movedInfo}`);
    document.dispatchEvent(new CustomEvent('cta:reload'));
  } catch (e) {
    setResult('err', `✗ ${(e as Error).message}`);
  }
}

export async function doDeleteDoc(
  url: string,
  setResult: (tone: ResultTone, text: string) => void,
) {
  if (!url) {
    setResult('err', 'Enter URL');
    return;
  }
  setResult('busy', 'Running gh api…');
  try {
    const data = await api.ghDelete(url);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      const deletedInfo = data.deleted !== undefined ? ` (${data.deleted} files deleted)` : '';
      setResult('warn', `⚠ ${data.warn}${deletedInfo}`);
      return;
    }
    const deletedInfo = data.deleted !== undefined ? ` (${data.deleted} files total)` : '';
    setResult('ok', `✓ Deleted${deletedInfo}`);
    document.dispatchEvent(new CustomEvent('cta:reload'));
  } catch (e) {
    setResult('err', `✗ ${(e as Error).message}`);
  }
}
