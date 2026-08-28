export const CATEGORY_ACTION_CREATE = '__create_category__';
export const CATEGORY_ACTION_DELETE = '__delete_category__';

export function createListOwner() {
  let activeOnly = true;
  let filterCategoryId = '';
  let categoryError = '';

  return {
    get activeOnly() {
      return activeOnly;
    },
    get filterCategoryId() {
      return filterCategoryId;
    },
    get categoryError() {
      return categoryError;
    },
    setCategoryError(message: string) {
      categoryError = message || '';
    },
    toggleActiveOnly() {
      activeOnly = !activeOnly;
    },
    setFilterCategoryId(id: string) {
      filterCategoryId = id || '';
      categoryError = '';
    },
    clearFilterCategory() {
      filterCategoryId = '';
    },
    resetError() {
      categoryError = '';
    },
  };
}
