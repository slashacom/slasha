import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpGet, httpPost } from '~/utils/http';
import { queryClient } from '~/utils/query-client';
import { setAuthToken } from '~/utils/jwt';
import type { User } from '~/models/user';

export interface AuthStatusResponse {
  has_admin: boolean;
}

export interface AuthMeResponse {
  user: User;
}

export interface AuthTokenResponse {
  token: string;
  user: User;
}

export function getAuthStatusOptions() {
  return queryOptions({
    queryKey: ['auth-status'],
    queryFn: () => httpGet<AuthStatusResponse>('auth/status', undefined),
  });
}

export function getAuthMeOptions() {
  return queryOptions({
    queryKey: ['auth-me'],
    queryFn: () =>
      httpGet<AuthMeResponse>('auth/me', undefined, {
        handleUnauthorized: true,
      }),
    retry: false,
  });
}

export function useSignup() {
  return useMutation(
    {
      mutationKey: ['signup'],
      mutationFn: (body: Record<string, any>) =>
        httpPost<AuthTokenResponse>('auth/signup', body),
      onSuccess: (data) => {
        setAuthToken(data.token);
        queryClient.invalidateQueries({
          queryKey: getAuthStatusOptions().queryKey,
        });
        queryClient.invalidateQueries({
          queryKey: getAuthMeOptions().queryKey,
        });
        queryClient.setQueryData(getAuthMeOptions().queryKey, {
          user: data.user,
        });
      },
    },
    queryClient
  );
}

export function useLogin() {
  return useMutation(
    {
      mutationKey: ['login'],
      mutationFn: (body: Record<string, any>) =>
        httpPost<AuthTokenResponse>('auth/login', body),
      onSuccess: (data) => {
        setAuthToken(data.token);
        queryClient.invalidateQueries({
          queryKey: getAuthMeOptions().queryKey,
        });
        queryClient.setQueryData(getAuthMeOptions().queryKey, {
          user: data.user,
        });
      },
    },
    queryClient
  );
}
