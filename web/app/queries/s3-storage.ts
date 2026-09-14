import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPatch, httpPost } from '~/utils/http';
import type { S3Storage } from '~/models/s3-storage';

export type CreateS3StoragePayload = {
  name: string;
  endpoint: string;
  bucket: string;
  region?: string;
  access_key_id: string;
  secret_access_key: string;
  force_path_style?: boolean;
};

export type UpdateS3StoragePayload = {
  name?: string;
  endpoint?: string;
  bucket?: string;
  region?: string;
  access_key_id?: string;
  secret_access_key?: string;
  force_path_style?: boolean;
};

export function getS3StoragesOptions() {
  return queryOptions({
    queryKey: ['s3-storages'],
    queryFn: () => httpGet<{ storages: S3Storage[] }>('s3-storages'),
  });
}

export function getS3StorageOptions(id: string) {
  return queryOptions({
    queryKey: ['s3-storages', id],
    queryFn: () => httpGet<{ storage: S3Storage }>(`s3-storages/${id}`),
  });
}

export function useCreateS3Storage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateS3StoragePayload) =>
      httpPost<{ storage: S3Storage }>('s3-storages', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['s3-storages'] });
    },
  });
}

export function useUpdateS3Storage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      payload,
    }: {
      id: string;
      payload: UpdateS3StoragePayload;
    }) => httpPatch<{ storage: S3Storage }>(`s3-storages/${id}`, payload),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['s3-storages'] });
      queryClient.invalidateQueries({
        queryKey: ['s3-storages', variables.id],
      });
    },
  });
}

export function useDeleteS3Storage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ deleted: boolean }>(`s3-storages/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['s3-storages'] });
    },
  });
}

export function useTestS3Storage() {
  return useMutation({
    mutationFn: (id: string) =>
      httpPost<{ ok: boolean }>(`s3-storages/${id}/test`, {}),
  });
}
