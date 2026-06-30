import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPatch, httpPost } from '~/utils/http';

export type GithubRepository = {
  id: number;
  full_name: string;
  default_branch: string;
  private: boolean;
  installation_id: number;
};

export type GithubInstallationInfo = {
  installation_id: number;
  configure_url: string;
  repositories_count: number;
};

export function getGithubStatusOptions() {
  return queryOptions({
    queryKey: ['github', 'status'],
    queryFn: () => httpGet<{ enabled: boolean }>('github-app/status'),
  });
}

export function useInstallGithub() {
  return useMutation({
    mutationFn: (data: { redirect_to: string }) =>
      httpPost<{ url: string }>('github-app/install', {
        redirect_to: data.redirect_to,
      }),
  });
}

export function getGithubRepositoriesOptions() {
  return queryOptions({
    queryKey: ['github', 'repositories'],
    queryFn: () =>
      httpGet<{
        installations: GithubInstallationInfo[];
        repositories: GithubRepository[];
      }>('github-app/repositories'),
  });
}

export function useRemoveGithubInstallation() {
  return useMutation({
    mutationFn: (installationId: number) =>
      httpDelete<void>(`github-app/installations/${installationId}`),
  });
}

export type GithubSetupStatus = {
  configured: boolean;
  app_id: string | null;
  created_at: string | null;
};

export function getGithubSetupStatusOptions() {
  return queryOptions({
    queryKey: ['github', 'app-setup', 'status'],
    queryFn: () => httpGet<GithubSetupStatus>('github-app/setup'),
  });
}

export function useBeginGithubSetup() {
  return useMutation({
    mutationFn: () =>
      httpPost<{ github_url: string; manifest: string }>(
        'github-app/setup/begin',
        {}
      ),
  });
}

export type UpdateGithubCredentialsPayload = {
  app_id: string;
  client_id: string;
  client_secret: string;
  private_key: string;
  webhook_secret: string;
};

export function useUpdateGithubCredentials() {
  return useMutation({
    mutationFn: (data: UpdateGithubCredentialsPayload) =>
      httpPatch<GithubSetupStatus>('github-app/setup', data),
  });
}

export function useDeleteGithubSetup() {
  return useMutation({
    mutationFn: () => httpDelete<void>('github-app/setup'),
  });
}
