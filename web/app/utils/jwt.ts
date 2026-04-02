import * as jose from 'jose';
import Cookies from 'js-cookie';

export const TOKEN_STORAGE_KEY = '__slasha_jwt__';

export type TokenPayload = {
  id: string;
  email: string;
  name: string;
  avatar: string;
};

export function decodeToken(token: string): TokenPayload {
  const claims = jose.decodeJwt(token);

  return claims as TokenPayload;
}

export function setAuthToken(token: string) {
  const isDev = import.meta.env.DEV;

  Cookies.set(TOKEN_STORAGE_KEY, token, {
    expires: 30,
    sameSite: 'lax',
    secure: !isDev,
    domain: isDev ? 'localhost' : '.slasha.app',
  });
}

export function getAuthToken() {
  return Cookies.get(TOKEN_STORAGE_KEY);
}

export function isLoggedIn() {
  const token = Cookies.get(TOKEN_STORAGE_KEY);

  return !!token;
}

export function getUser() {
  const token = Cookies.get(TOKEN_STORAGE_KEY);

  if (!token) {
    return null;
  }

  return decodeToken(token);
}

export function removeAuthToken() {
  const isDev = import.meta.env.DEV;

  Cookies.remove(TOKEN_STORAGE_KEY, {
    domain: isDev ? 'localhost' : '.slasha.app',
  });
}
