import { renderKbMdBody } from '../../ui/viewer/highlight.ts';
import { hydrateKbRelativeImages } from './images.ts';

export async function paintKbMdBody(text: string, bodyEl?: HTMLElement | null) {
  await renderKbMdBody(text, bodyEl);
  const body = bodyEl ?? document.getElementById('kb-md-body');
  if (body) await hydrateKbRelativeImages(body);
}
