import { showOsNotification } from '../../host/api.ts';
import { composeWorkbenchScheme, type WorkbenchEnvelope } from '../../router/scheme.ts';
import { refreshNotesIndex } from './reload-index.ts';

export async function handleNotesOsNotifyEnvelope(
  envelope: WorkbenchEnvelope,
): Promise<void> {
  if (!envelope || typeof envelope !== 'object') return;
  if (envelope.business !== 'notes') return;
  if (envelope.action === 'create' || envelope.action === 'update') {
    await refreshNotesIndex();
  }
  if (envelope.action !== 'create') return;
  const scheme = composeWorkbenchScheme(envelope);
  if (!scheme) return;
  try {
    await showOsNotification({
      title: 'New note',
      body: 'A note was added',
      scheme,
    });
  } catch {
    // AC-7: command failure does not prompt the user.
  }
}
