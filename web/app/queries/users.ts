import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpGet, httpPost, httpPatch, httpDelete } from '~/utils/http';
import type { User } from '~/models/user';

export function getUsersOptions() {
  return queryOptions({
    queryKey: ['users'],
    queryFn: () => httpGet<{ users: User[] }>('users'),
  });
}

export function getUserOptions(id: string) {
  return queryOptions({
    queryKey: ['users', id],
    queryFn: () => httpGet<{ user: User }>(`users/${id}`),
  });
}

export function useCreateUser() {
  return useMutation({
    mutationFn: (payload: Record<string, any>) =>
      httpPost<{ user: User }>('users', payload),
  });
}

export function useUpdateUser(id: string) {
  return useMutation({
    mutationFn: (payload: Record<string, any>) =>
      httpPatch<{ user: User }>(`users/${id}`, payload),
  });
}

export function useDeleteUser() {
  return useMutation({
    mutationFn: (id: string) => httpDelete<{ deleted: boolean }>(`users/${id}`),
  });
}
