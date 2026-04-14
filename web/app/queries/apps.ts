import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost } from '~/utils/http';
import type { App } from '~/models/app';

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
