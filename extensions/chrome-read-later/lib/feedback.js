export function badgeFeedbackForResult(result) {
  if (result.ok) {
    return { badgeText: 'OK', title: '已保存到待读' };
  }

  if (result.status === 0) {
    return { badgeText: '!', title: '请先启动 Workbench' };
  }

  if (result.code === 'read_later_recent_duplicate') {
    return { badgeText: '!', title: '24小时内已保存' };
  }

  return {
    badgeText: '!',
    title: result.error || '保存失败',
  };
}
