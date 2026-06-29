import { redirect } from 'react-router';

export async function clientLoader() {
  throw redirect('/alerts');
}

export default function AlertsNotificationsRedirectPage() {
  return null;
}
