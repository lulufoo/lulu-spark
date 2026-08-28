export function fmtDate(isoStr: string | undefined) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('en-US', {
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoStr;
  }
}

export function fmtDateFull(isoStr: string | undefined) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('en-US', {
      year: 'numeric',
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoStr;
  }
}

export function pickXSelection<T extends { handle?: string }>(
  xUsers: T[],
  savedHandle: string,
): { xSelHandle: string; xSelUser: T | undefined } {
  const xSelHandle =
    savedHandle && xUsers.find((u) => u.handle === savedHandle)
      ? savedHandle
      : xUsers[0]
        ? xUsers[0].handle || ''
        : '';
  const xSelUser = xUsers.find((u) => u.handle === xSelHandle) || xUsers[0];
  return { xSelHandle, xSelUser };
}
