import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';

export type BackupConfig = {
  enabled: boolean;
  db_path: string;
  bucket: string;
  endpoint: string;
  path_prefix: string | null;
  access_key_id: string;
  secret_set: boolean;
  restore_pending: boolean;
  last_synced_at: string | null;
};

export type VolumeInfo = {
  path: string;
  managed: boolean;
  exists: boolean;
  size_bytes: number | null;
};

export type SaveBackupPayload = {
  appSlug: string;
  enabled: boolean;
  db_path: string;
  bucket: string;
  endpoint: string;
  path_prefix?: string;
  access_key_id: string;
  secret_access_key?: string;
};

export function getBackupOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'backups'],
    queryFn: () =>
      httpGet<{ backup: BackupConfig | null }>(`apps/${appSlug}/backups`),
  });
}

export function getVolumesOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'volumes'],
    queryFn: () =>
      httpGet<{ volumes: VolumeInfo[] }>(`apps/${appSlug}/volumes`),
  });
}

export function useSaveBackup() {
  return useMutation({
    mutationFn: (payload: SaveBackupPayload) => {
      const { appSlug, ...body } = payload;
      return httpPut<{ backup: BackupConfig }>(`apps/${appSlug}/backups`, body);
    },
  });
}

export function useRestoreBackup() {
  return useMutation({
    mutationFn: (appSlug: string) =>
      httpPost(`apps/${appSlug}/backups/restore`, {}),
  });
}

export function useDeleteBackup() {
  return useMutation({
    mutationFn: (appSlug: string) => httpDelete(`apps/${appSlug}/backups`),
  });
}

export type BackupStatus = {
  enabled: boolean;
  restore_pending: boolean;
  web_running: boolean;
  last_synced_at: string | null;
  last_checked_at: string | null;
  healthy: boolean | null;
  health_error: string | null;
};

export function getBackupStatusOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'backups', 'status'],
    queryFn: () =>
      httpGet<{ status: BackupStatus }>(`apps/${appSlug}/backups/status`),
  });
}

export type ReplicaHealth = {
  healthy: boolean;
  health_error: string | null;
  last_synced_at: string | null;
};

// Probes the object-storage replica directly (reachability + freshness). This
// runs a one-shot container server-side, so it's polled on a slow cadence, not
// the cheap status poll.
export function getReplicaHealthOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'backups', 'probe'],
    queryFn: () =>
      httpPost<ReplicaHealth>(`apps/${appSlug}/backups/status/refresh`, {}),
  });
}
