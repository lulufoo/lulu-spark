import { closeSparkSearch } from '../../notes/ui/search.tsx';

export function applySearchNavChrome(routeName: string) {
  closeSparkSearch();

  const wbWrap = document.getElementById('gs-wb-wrap');
  const wbInput = document.getElementById('gs-wb-input') as HTMLInputElement | null;
  const onSpark = routeName === 'spark';

  if (wbWrap) wbWrap.hidden = !onSpark;
  if (wbInput) wbInput.disabled = !onSpark;
}
