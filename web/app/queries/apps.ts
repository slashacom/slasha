import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { App, AppEnvVar } from '~/models/app';

export function getAppsOptions() {
  return queryOptions({
    queryKey: ['apps'],
    queryFn: () => httpGet<{ apps: App[] }>('apps'),
  });
}

export function getAppOptions(slug: string) {
  return queryOptions({
    queryKey: ['apps', slug],
    queryFn: () => httpGet<{ app: App }>(`apps/${slug}`),
  });
}

export function useCreateApp() {
  return useMutation({
    mutationFn: (data: { name: string }) =>
      httpPost<{ app: App }>('apps', data),
  });
}

export function useDeleteApp() {
  return useMutation({
    mutationFn: (slug: string) => httpDelete(`apps/${slug}`),
  });
}

export function getAppEnvVarsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'env-vars'],
    queryFn: () =>
      httpGet<{ env_vars: Record<string, string> }>(`apps/${appSlug}/env`),
  });
}

export function useUpdateAppEnvVars() {
  return useMutation({
    mutationFn: (data: { appSlug: string; vars: Record<string, string> }) =>
      httpPut<{ env_vars: Record<string, string> }>(
        `apps/${data.appSlug}/env`,
        {
          vars: data.vars,
        }
      ),
  });
}
