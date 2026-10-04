import { showOsNotification } from '../../host/api.ts';
import { composeSparkScheme, type SparkEnvelope } from '../../router/scheme.ts';

export async function handleReadLaterOsNotifyEnvelope(
  envelope: SparkEnvelope,
): Promise<void> {
  if (!envelope || typeof envelope !== 'object') return;
  if (envelope.business !== 'read_later' || envelope.action !== 'create') return;
  const scheme = composeSparkScheme(envelope);
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
