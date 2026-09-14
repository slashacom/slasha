import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type {
  ServiceBackup,
  ServiceBackupConfig,
} from '~/models/service-backup';

export type UpdateServiceBackupConfigPayload = {
  enabled: boolean;
  schedule: string;
  timezone: string;
  retention_count: number;
  s3_storage_id?: string | null;
  keep_local?: boolean;
};

export function getServiceBackupConfigOptions(
  appSlug: string,
  serviceId: string
) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services', serviceId, 'backup-config'],
    queryFn: () =>
      httpGet<{ config: ServiceBackupConfig | null }>(
        `apps/${appSlug}/services/${serviceId}/backup-config`
      ),
  });
}

export function useUpdateServiceBackupConfig(
  appSlug: string,
  serviceId: string
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateServiceBackupConfigPayload) =>
      httpPut<{ config: ServiceBackupConfig }>(
        `apps/${appSlug}/services/${serviceId}/backup-config`,
        payload
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'backup-config'],
      });
    },
  });
}

export function getServiceBackupsOptions(appSlug: string, serviceId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services', serviceId, 'backups'],
    queryFn: () =>
      httpGet<{ backups: ServiceBackup[] }>(
        `apps/${appSlug}/services/${serviceId}/backups`
      ),
  });
}

export function useTriggerServiceBackup(appSlug: string, serviceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      httpPost<{ backup: ServiceBackup }>(
        `apps/${appSlug}/services/${serviceId}/backups`,
        {}
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'backups'],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'backup-config'],
      });
    },
  });
}

export function useRestoreServiceBackup(appSlug: string, serviceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (backupId: string) =>
      httpPost<{ restored: boolean }>(
        `apps/${appSlug}/services/${serviceId}/backups/${backupId}/restore`,
        {}
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'backups'],
      });
    },
  });
}

export function useDeleteServiceBackup(appSlug: string, serviceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (backupId: string) =>
      httpDelete<{ deleted: boolean }>(
        `apps/${appSlug}/services/${serviceId}/backups/${backupId}`
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'backups'],
      });
    },
  });
}
