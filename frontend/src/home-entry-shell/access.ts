type HomeEntryShell = { forceRecoverA?: (reason: string) => void } | null;

let homeEntryShell: HomeEntryShell = null;

export function setHomeEntryShell(shell: HomeEntryShell) {
  homeEntryShell = shell;
}

export function getHomeEntryShell() {
  return homeEntryShell;
}
