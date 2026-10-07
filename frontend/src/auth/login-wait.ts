let inFlightPassId: string | null = null;

export function beginAuthLoginWait(id: string): void {
  inFlightPassId = id;
}

export function consumeMatchingAuthLoginPass(id: string | null): boolean {
  if (id === null || inFlightPassId === null || id !== inFlightPassId) {
    return false;
  }
  inFlightPassId = null;
  return true;
}
