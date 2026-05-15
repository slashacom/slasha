import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { App } from '~/models/app';
import type { AppScale } from '~/models/app_scale';
import type { AppDomain } from '~/models/app';

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

export type EnvSuggestionService = {
  name: string;
  env_keys: string[];
};

export function getAppEnvSuggestionsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'env-suggestions'],
    queryFn: () =>
      httpGet<{ services: EnvSuggestionService[] }>(
        `apps/${appSlug}/env/suggestions`
      ),
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

export function getScalesOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'scales'],
    queryFn: () => httpGet<{ scales: AppScale[] }>(`apps/${appSlug}/scales`),
  });
}

export function getAppDomainsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'domains'],
    queryFn: () => httpGet<{ domains: AppDomain[] }>(`apps/${appSlug}/domains`),
  });
}

export function useAddAppDomain() {
  return useMutation({
    mutationFn: (data: { appSlug: string; domain: string }) =>
      httpPost<{ domain: AppDomain }>(`apps/${data.appSlug}/domains`, {
        domain: data.domain,
      }),
  });
}

export function useDeleteAppDomain() {
  return useMutation({
    mutationFn: (data: { appSlug: string; domainId: string }) =>
      httpDelete(`apps/${data.appSlug}/domains/${data.domainId}`),
  });
}
