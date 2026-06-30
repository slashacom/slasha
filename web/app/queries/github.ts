import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost } from '~/utils/http';

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
    queryFn: () => httpGet<{ enabled: boolean }>('github/status'),
  });
}

export function useInstallGithub() {
  return useMutation({
    mutationFn: (data: { redirect_to: string }) =>
      httpPost<{ url: string }>('github/install', {
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
      }>('github/repositories'),
  });
}

export function useRemoveGithubInstallation() {
  return useMutation({
    mutationFn: (installationId: number) =>
      httpDelete<void>(`github/installations/${installationId}`),
  });
}
