export function isXCollapsed() {
  return localStorage.getItem('cta_x_collapsed') === '1';
}

export function getXHandle() {
  return localStorage.getItem('cta_x_handle') || '';
}

export function saveXCollapsed(collapsed: boolean) {
  localStorage.setItem('cta_x_collapsed', collapsed ? '1' : '0');
}

export function saveXHandle(handle: string) {
  localStorage.setItem('cta_x_handle', handle);
}
