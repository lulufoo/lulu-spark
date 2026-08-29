import { closeWorkbenchSearch } from '../../notes/ui/search.tsx';
import { closeKnowledgeSearch } from '../../knowledge/ui/search.tsx';

export function applySearchNavChrome(routeName: string) {
  closeWorkbenchSearch();
  closeKnowledgeSearch();

  const onHome = routeName === 'home';
  const onWorkbench = routeName === 'workbench';
  const onKnowledge = routeName === 'knowledge-doc';

  const wbWrap = document.getElementById('gs-wb-wrap');
  const kbWrap = document.getElementById('gs-kb-wrap');
  const wbInput = document.getElementById('gs-wb-input') as HTMLInputElement | null;
  const kbInput = document.getElementById('gs-kb-input') as HTMLInputElement | null;

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
  } else if (onKnowledge) {
    if (wbWrap) wbWrap.hidden = true;
    if (kbWrap) kbWrap.hidden = false;
    if (wbInput) wbInput.disabled = true;
    if (kbInput) kbInput.disabled = false;
  }
}
