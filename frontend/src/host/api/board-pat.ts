import { readGet, writePost } from './transport.ts';

export async function fetchBoardPatStatus() {
  return readGet('/api/board-pat');
}

export async function setBoardPat(pat: string) {
  return writePost('/api/board-pat', { pat });
}

export async function deleteBoardPat() {
  return writePost('/api/board-pat/delete', {});
}
