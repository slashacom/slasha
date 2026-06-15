import Cookies from 'js-cookie';

export const TOKEN_STORAGE_KEY = '__slasha_jwt__';

export function setAuthToken(token: string) {
  Cookies.set(TOKEN_STORAGE_KEY, token, {
    expires: 30,
    sameSite: 'lax',
    secure: window.location.protocol === 'https:',
  });
}

export function getAuthToken() {
  return Cookies.get(TOKEN_STORAGE_KEY);
}

export function isLoggedIn() {
  const token = Cookies.get(TOKEN_STORAGE_KEY);

  return !!token;
}

export function removeAuthToken() {
  Cookies.remove(TOKEN_STORAGE_KEY);
}
