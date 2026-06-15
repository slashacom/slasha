import { redirect } from 'react-router';

export async function clientLoader() {
  throw redirect('/settings/account');
}

export default function SettingsIndex() {
  return null;
}
