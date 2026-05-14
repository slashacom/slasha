import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpGet, httpPost, httpDelete } from '~/utils/http';
import type { Deployment } from '~/models/deployment';
import type {
  ProcessContainer,
  ProcessType,
} from '~/models/app_scale';

export type CommitInfo = {
  sha: string;
  message: string;
};

export function getCommitsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'commits'],
    queryFn: () =>
      httpGet<{ commits: CommitInfo[] }>(`apps/${appSlug}/commits`),
  });
}

export function getDeploymentsOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'deployments'],
    queryFn: () =>
      httpGet<{ deployments: Deployment[] }>(`apps/${appSlug}/deployments`),
  });
}

export function getDeploymentOptions(appSlug: string, deploymentId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'deployments', deploymentId],
    queryFn: () =>
      httpGet<{ deployment: Deployment }>(
        `apps/${appSlug}/deployments/${deploymentId}`
      ),
  });
}

export function getProcessesOptions(appSlug: string, deploymentId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'deployments', deploymentId, 'processes'],
    queryFn: () =>
      httpGet<{ processes: ProcessContainer[] }>(
        `apps/${appSlug}/deployments/${deploymentId}/processes`
      ),
  });
}

export function useTriggerDeploy() {
  return useMutation({
    mutationFn: (data: { appSlug: string; commitSha?: string }) =>
      httpPost<{ deployment: Deployment }>(`apps/${data.appSlug}/deployments`, {
        commit_sha: data.commitSha,
      }),
  });
}

export function useStopDeployment() {
  return useMutation({
    mutationFn: (data: { appSlug: string; deploymentId: string }) =>
      httpPost<{ stopped: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/stop`,
        {}
      ),
  });
}

export function useDeleteDeployment() {
  return useMutation({
    mutationFn: (data: { appSlug: string; deploymentId: string }) =>
      httpDelete<{ deleted: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}`
      ),
  });
}

export function useRestartDeployment() {
  return useMutation({
    mutationFn: (data: { appSlug: string; deploymentId: string }) =>
      httpPost<{ restarted: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/restart`,
        {}
      ),
  });
}

export function useRedeployDeployment() {
  return useMutation({
    mutationFn: (data: { appSlug: string; deploymentId: string }) =>
      httpPost<{ deployment: Deployment }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/redeploy`,
        {}
      ),
  });
}
export function useScaleDeployment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      appSlug: string;
      deploymentId: string;
      processType: ProcessType;
      count: number;
    }) =>
      httpPost<{ scaled: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/scale`,
        {
          process_type: data.processType,
          count: data.count,
        }
      ),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: [
          'apps',
          variables.appSlug,
          'deployments',
          variables.deploymentId,
          'processes',
        ],
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', variables.appSlug, 'scales'],
      });
    },
  });
}
