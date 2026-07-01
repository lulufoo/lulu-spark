export function badgeFeedbackForResult(result) {
  if (result.ok) {
    return { badgeText: 'OK', title: '已保存到待读' };
  }

  if (result.status === 503) {
    return { badgeText: '!', title: '请先启动 Workbench' };
  }

  if (result.status === 0) {
    return { badgeText: '!', title: '网络错误，请检查 Workbench' };
  }

  return {
    badgeText: '!',
    title: result.error || '保存失败',
  };
}
