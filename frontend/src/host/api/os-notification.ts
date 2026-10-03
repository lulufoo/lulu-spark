import { writePost } from './transport.ts';

export async function showOsNotification(input: {
  title: string;
  body: string;
  scheme: string;
}) {
  return writePost('/api/show-os-notification', input);
}
