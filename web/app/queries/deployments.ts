import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpGet, httpPost, httpDelete } from '~/utils/http';
import type { Deployment } from '~/models/deployment';
import type { ProcessContainer, ProcessType } from '~/models/app-scale';

export type CommitInfo = {
  sha: string;
  message: string;
};

type TriggerDeployPayload = { appSlug: string; commitSha?: string };

type DeploymentRef = { appSlug: string; deploymentId: string };

type ScaleDeploymentPayload = {
  appSlug: string;
  deploymentId: string;
  processType: ProcessType;
  count: number;
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

function useInvalidateAppQueries() {
  const queryClient = useQueryClient();
  return (appSlug: string) => {
    queryClient.invalidateQueries({ queryKey: ['apps', appSlug] });
    queryClient.invalidateQueries({ queryKey: ['apps'], exact: true });
  };
}

export function useTriggerDeploy() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: TriggerDeployPayload) =>
      httpPost<{ deployment: Deployment }>(`apps/${data.appSlug}/deployments`, {
        commit_sha: data.commitSha,
      }),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useStopDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpPost<{ stopped: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/stop`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useCancelDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpPost<{ cancelled: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/cancel`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useDeleteDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpDelete<{ deleted: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}`
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useRestartDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpPost<{ restarted: boolean }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/restart`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useRedeployDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpPost<{ deployment: Deployment }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/redeploy`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useRollbackDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: DeploymentRef) =>
      httpPost<{ deployment: Deployment }>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/rollback`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useScaleDeployment() {
  const invalidate = useInvalidateAppQueries();
  return useMutation({
    mutationFn: (data: ScaleDeploymentPayload) =>
      httpPost<void>(
        `apps/${data.appSlug}/deployments/${data.deploymentId}/scale`,
        {
          process_type: data.processType,
          count: data.count,
        }
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}
