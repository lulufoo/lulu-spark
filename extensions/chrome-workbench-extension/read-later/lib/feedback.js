const BADGE_GREEN = '#22c55e';
const BADGE_YELLOW = '#eab308';
const BADGE_RED = '#ef4444';

export function badgeFeedbackForResult(result) {
  if (result.ok) {
    return { badgeText: 'OK', title: '已保存到待读', badgeColor: BADGE_GREEN };
  }

  if (result.status === 0) {
    return {
      badgeText: '!',
      title: '请先启动 Lulu Spark',
      badgeColor: BADGE_RED,
    };
  }

  if (result.code === 'read_later_recent_duplicate') {
    return {
      badgeText: '!',
      title: '24小时内已保存',
      badgeColor: BADGE_YELLOW,
    };
  }

  return {
    badgeText: '!',
    title: result.error || '保存失败',
    badgeColor: BADGE_RED,
  };
}
