import { redirect } from 'react-router';
import { isLoggedIn } from '~/utils/jwt';

export async function clientLoader() {
  if (isLoggedIn()) {
    throw redirect('/apps');
  } else {
    throw redirect('/login');
  }
}

export default function Index() {
  return null;
}
