/** Copy one chat message. Returns false when there is nothing to write. */
export async function copyMessageText(text: string) {
  const value = String(text ?? '');
  if (!value) return false;
  if (!navigator.clipboard?.writeText) return false;
  try {
    await navigator.clipboard.writeText(value);
    return true;
  } catch {
    return false;
  }
}
