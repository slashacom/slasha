import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpGet, httpPost, httpDelete } from '~/utils/http';
import type { SshKey } from '~/models/ssh-key';

type CreateSshKeyPayload = { title?: string; public_key: string };

export function getSshKeysOptions() {
  return queryOptions({
    queryKey: ['ssh-keys'],
    queryFn: () => httpGet<{ keys: SshKey[] }>('ssh-keys'),
  });
}

export function useCreateSshKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateSshKeyPayload) =>
      httpPost<SshKey>('ssh-keys', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ssh-keys'] });
    },
  });
}

export function useDeleteSshKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ status: string }>(`ssh-keys/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ssh-keys'] });
    },
  });
}
