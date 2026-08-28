export { notesDocKey, knowledgeDocKey, todosDocKey } from './identity.ts';
export { renderDocMarkdown, setDocEditMode } from './view.tsx';
export {
  applyCachedHighlights,
  initDocHighlightOverlay,
  cleanupDocHighlightOverlay,
} from './highlights.ts';
export { bindTodoDocHighlights } from './todo-bind.ts';
