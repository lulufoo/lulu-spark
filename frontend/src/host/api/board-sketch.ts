import { readGet } from './transport.ts';

export async function fetchBoardSketch(url: string) {
  return readGet(`/api/board-sketch?url=${encodeURIComponent(url)}`);
}
