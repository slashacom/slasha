import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { App, AppSource } from '~/models/app';
import type { AppScale } from '~/models/app-scale';
import type { AppDomain } from '~/models/app';
import type { DomainHealth } from '~/models/domain-health';

import type { AppMetrics } from '~/models/app-metrics';
import type { GitConnection } from '~/models/connection';

export type AppListItem = {
  app: App;
  url: string;
  runtime_status: 'idle' | 'deploying' | 'running' | 'failed';
};

type CreateAppPayload<Source extends AppSource = AppSource> = {
  name: string;
  source: Source;
  node_id?: string;
} & (Source extends 'github'
  ? { installation_id: number; repository_id: number }
  : Source extends 'git'
    ? { url: string; branch?: string }
    : Record<never, never>);

export type GitAppConnection = Pick<GitConnection, 'clone_url'>;

export type GithubConnectionRepository = {
  full_name: string;
  html_url: string;
  default_branch: string;
};

export type GithubAppConnection = {
  installation_id: number;
  repository_id: number;
  repository: GithubConnectionRepository | null;
};

export type AppConnection = GitAppConnection | GithubAppConnection;

type UpdateAppEnvVarsPayload = {
  appSlug: string;
  vars: Record<string, string>;
};

type AddAppDomainPayload = { appSlug: string; domain: string };

type DeleteAppDomainPayload = { appSlug: string; domainId: string };

type UpdateAppSettingsPayload = {
  appSlug: string;
  name?: string;
  auto_deploy?: boolean;
};

export function getAppsOptions() {
  return queryOptions({
    queryKey: ['apps'],
    queryFn: () => httpGet<{ apps: AppListItem[] }>('apps'),
  });
}

export function getCheckSlugOptions(name: string) {
  return queryOptions({
    queryKey: ['check-slug', name],
    queryFn: () =>
      httpGet<{ slug: string; available: boolean }>(
        `apps/check-slug?name=${encodeURIComponent(name)}`
      ),
  });
}

export function getAppOptions(slug: string) {
  return queryOptions({
    queryKey: ['apps', slug],
    queryFn: () =>
      httpGet<{
        app: App;
        url: string;
        runtime_status: string;
      }>(`apps/${slug}`),
  });
}

export function getAppConnectionOptions(slug: string) {
  return queryOptions({
    queryKey: ['apps', slug, 'connection'],
    queryFn: () =>
      httpGet<{ connection?: AppConnection }>(`apps/${slug}/connection`),
  });
}

export function useCreateApp() {
  return useMutation({
    mutationFn: (data: CreateAppPayload) =>
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
    mutationFn: (data: UpdateAppEnvVarsPayload) =>
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

export function getAppDomainsHealthOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'domains', 'health'],
    queryFn: () =>
      httpGet<{ health: DomainHealth[] }>(`apps/${appSlug}/domains/health`),
  });
}

export function useAddAppDomain() {
  return useMutation({
    mutationFn: (data: AddAppDomainPayload) =>
      httpPost<{ domain: AppDomain }>(`apps/${data.appSlug}/domains`, {
        domain: data.domain,
      }),
  });
}

export function useDeleteAppDomain() {
  return useMutation({
    mutationFn: (data: DeleteAppDomainPayload) =>
      httpDelete(`apps/${data.appSlug}/domains/${data.domainId}`),
  });
}

export function getAppMetricsOptions(
  appSlug: string,
  start?: Date,
  end?: Date
) {
  let queryParams = new URLSearchParams();
  if (start) queryParams.append('start', start.toISOString());
  if (end) queryParams.append('end', end.toISOString());
  const qs = queryParams.toString();

  return queryOptions({
    queryKey: ['apps', appSlug, 'metrics', { start, end }],
    queryFn: () =>
      httpGet<{ metrics: AppMetrics[] }>(
        `apps/${appSlug}/metrics${qs ? `?${qs}` : ''}`
      ),
  });
}

export function useUpdateAppSettings() {
  return useMutation({
    mutationFn: (data: UpdateAppSettingsPayload) =>
      httpPut<{ success: boolean }>(`apps/${data.appSlug}/settings`, {
        name: data.name,
        auto_deploy: data.auto_deploy,
      }),
  });
}

export function useReconnectGithub() {
  return useMutation({
    mutationFn: (data: {
      appSlug: string;
      installation_id: number;
      repository_id: number;
    }) =>
      httpPut<void>(`apps/${data.appSlug}/connection/github`, {
        installation_id: data.installation_id,
        repository_id: data.repository_id,
      }),
  });
}

export function useDisconnectGithub() {
  return useMutation({
    mutationFn: (appSlug: string) =>
      httpDelete<void>(`apps/${appSlug}/connection/github`),
  });
}

export function useSyncApp() {
  return useMutation({
    mutationFn: (appSlug: string) => httpPost<void>(`apps/${appSlug}/sync`, {}),
  });
}

export function useMoveAppNode() {
  return useMutation({
    mutationFn: (data: { appSlug: string; node_id: string }) =>
      httpPut<{ app: App }>(`apps/${data.appSlug}/node`, {
        node_id: data.node_id,
      }),
  });
}
