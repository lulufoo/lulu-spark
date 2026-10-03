import { showOsNotification } from '../../host/api.ts';
import { composeWorkbenchScheme, type WorkbenchEnvelope } from '../../router/scheme.ts';

export async function handleReadLaterOsNotifyEnvelope(
  envelope: WorkbenchEnvelope,
): Promise<void> {
  if (!envelope || typeof envelope !== 'object') return;
  if (envelope.business !== 'read_later' || envelope.action !== 'create') return;
  const scheme = composeWorkbenchScheme(envelope);
  if (!scheme) return;
  try {
    await showOsNotification({
      title: 'Read Later',
      body: 'A link was saved',
      scheme,
    });
  } catch {
    // AC-7: command failure does not prompt the user.
  }
}
