export {
  createTodosPageLifecycle,
  onTodosPageEnter,
  onMasterSelectionChange,
  onTodosPageLeave,
} from './lifecycle.js';

/**
 * t4 / N1: `open_ai_assistant(masterTaskId)` is not Todos executable success main path.
 * t6: no in-page Assistant Present entry; Binding is via Binding Contract Set/Reset.
 */
export const TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED = true;
/**
 * SK-4 / T-acceptance: Todos-side parity acceptance ids (P1–P6 + N1/N2 only).
 * Does not include complete/abandon/batch/cross-plan/global entry.
 */
export const TODOS_PARITY_ACCEPTANCE = Object.freeze([
  'P1',
  'P2',
  'P3',
  'P4',
  'P5',
  'P6',
  'N1',
  'N2',
]);

export { DEFAULT_PLAN_CATEGORY_ID } from './host.js';
export {
  loadTodoTasks,
  createTodoTask,
  deleteTodoTask,
  addPlanSub,
  deletePlanSub,
  readPlanMd,
  updatePlanMd,
  completePlan,
  abandonPlanSub,
  updatePlanSub,
  updatePlanMasterTitle,
  setPlanMasterStatus,
  listPlanCategories,
  createPlanCategory,
  deletePlanCategory,
  setPlanCategory,
  listPlanAttachments,
  stagePlanAttachmentSource,
  addPlanAttachment,
  readPlanAttachment,
  savePlanAttachment,
  deletePlanAttachment,
  listPlanComments,
  addPlanComment,
  updatePlanComment,
  deletePlanComment,
  pickLocalMarkdownFile,
} from './host.js';

export {
  copySubIdPair,
  formatMasterCopyText,
  formatTodoTaskStatus,
} from './format.js';

export { syncCategoryFilterWidth } from './list.js';
export { renderSubRow, renderSubDetail, renderSubDetailPane } from './detail.js';
export { mountTodoTaskSplit } from './page.js';
