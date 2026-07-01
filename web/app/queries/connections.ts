import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import {
  httpDelete,
  httpGet,
  httpPatch,
  httpPost,
  httpPut,
} from '~/utils/http';
import { queryClient } from '~/utils/query-client';

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
    queryFn: () => httpGet<{ enabled: boolean }>('connections/github/status'),
  });
}

export function useInstallGithub() {
  return useMutation({
    mutationFn: (data: { redirect_to: string }) =>
      httpPost<{ url: string }>('connections/github/install', {
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
      }>('connections/github/repositories'),
  });
}

export function useRemoveGithubInstallation() {
  return useMutation({
    mutationFn: (installationId: number) =>
      httpDelete<void>(`connections/github/installations/${installationId}`),
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
    queryFn: () => httpGet<GithubSetupStatus>('connections/github/setup'),
  });
}

export function useBeginGithubSetup() {
  return useMutation({
    mutationFn: () =>
      httpPost<{ github_url: string; manifest: string }>(
        'connections/github/setup/begin',
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
      httpPatch<GithubSetupStatus>('connections/github/setup', data),
  });
}

export function useDeleteGithubSetup() {
  return useMutation({
    mutationFn: () => httpDelete<void>('connections/github/setup'),
  });
}

export type GitRemoteResponse = {
  default_branch: string | null;
  branches: string[];
};

export function useGetRemoteBranchesQuery(url: string) {
  return useQuery({
    queryKey: ['git', 'remote-branches', url],
    queryFn: () =>
      httpGet<GitRemoteResponse>(
        `connections/git/remote-branches?url=${encodeURIComponent(url)}`
      ),
    enabled:
      url.trim().length > 0 &&
      (url.startsWith('http://') || url.startsWith('https://')),
    retry: false,
  });
}

export type GithubRemoteResponse = {
  default_branch: string;
  branches: string[];
};

export function useGetGithubBranchesQuery(
  installationId: number | undefined,
  repositoryId: number | undefined
) {
  return useQuery({
    queryKey: ['github', 'branches', installationId, repositoryId],
    queryFn: () =>
      httpGet<GithubRemoteResponse>(
        `connections/github/installations/${installationId}/repositories/${repositoryId}/branches`
      ),
    enabled: installationId !== undefined && repositoryId !== undefined,
    retry: false,
  });
}

export function useUpdateConnectionBranch(slug: string) {
  return useMutation({
    mutationFn: (branch: string) =>
      httpPut<void>(`apps/${slug}/connection/branch`, { branch }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apps', slug, 'connection'] });
    },
  });
}
