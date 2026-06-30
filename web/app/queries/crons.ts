import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import { queryClient } from '~/utils/query-client';
import type { CronJob, CronRun } from '~/models/cron';

type CronPayload = {
  name: string;
  schedule: string;
  command: string;
  timezone: string;
  enabled: boolean;
  timeout_secs: number;
};

export function getCronsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'crons'],
    queryFn: () => httpGet<{ crons: CronJob[] }>(`apps/${appSlug}/crons`),
  });
}

export function getCronRunsOptions(appSlug: string, cronId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'crons', cronId, 'runs'],
    queryFn: () =>
      httpGet<{ runs: CronRun[] }>(`apps/${appSlug}/crons/${cronId}/runs`),
  });
}

export function getCronPreviewOptions(
  appSlug: string,
  schedule: string,
  timezone: string
) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'crons', 'preview', schedule, timezone],
    queryFn: () =>
      httpPost<{ next_runs: string[] }>(`apps/${appSlug}/crons/preview`, {
        schedule,
        timezone,
      }),
    retry: false,
  });
}

export function useCreateCron(appSlug: string) {
  return useMutation({
    mutationFn: (payload: CronPayload) =>
      httpPost<{ cron: CronJob }>(`apps/${appSlug}/crons`, payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'crons'] });
    },
  });
}

export function useUpdateCron(appSlug: string) {
  return useMutation({
    mutationFn: (payload: { id: string; data: CronPayload }) =>
      httpPut<{ cron: CronJob }>(
        `apps/${appSlug}/crons/${payload.id}`,
        payload.data
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'crons'] });
    },
  });
}

export function useDeleteCron(appSlug: string) {
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ deleted: boolean }>(`apps/${appSlug}/crons/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'crons'] });
    },
  });
}

export function useRunCron(appSlug: string) {
  return useMutation({
    mutationFn: (id: string) =>
      httpPost<{ run: CronRun }>(`apps/${appSlug}/crons/${id}/run`, {}),
    onSuccess: (_result, id) => {
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'crons', id, 'runs'],
      });
    },
  });
}
