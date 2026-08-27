import { closeWorkbenchSearch } from '../notes/search.js';
import { closeCorpusSearch } from '../corpus/corpus-search.js';

export function applySearchNavChrome(routeName) {
  closeWorkbenchSearch();
  closeCorpusSearch();

  const onHome = routeName === 'home';
  const onWorkbench = routeName === 'workbench';
  const onCorpus = routeName === 'corpus-doc';

  const wbWrap = document.getElementById('gs-wb-wrap');
  const kbWrap = document.getElementById('gs-kb-wrap');
  const wbInput = document.getElementById('gs-wb-input');
  const kbInput = document.getElementById('gs-kb-input');

  if (onHome) {
    if (wbWrap) wbWrap.hidden = true;
    if (kbWrap) kbWrap.hidden = true;
    if (wbInput) wbInput.disabled = true;
    if (kbInput) kbInput.disabled = true;
  } else if (onWorkbench) {
    if (wbWrap) wbWrap.hidden = false;
    if (kbWrap) kbWrap.hidden = true;
    if (wbInput) wbInput.disabled = false;
    if (kbInput) kbInput.disabled = true;
  } else if (onCorpus) {
    if (wbWrap) wbWrap.hidden = true;
    if (kbWrap) kbWrap.hidden = false;
    if (wbInput) wbInput.disabled = true;
    if (kbInput) kbInput.disabled = false;
  }
}
