import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpGet, httpPost, httpDelete, httpPut } from '~/utils/http';
import type { Service, ServiceKind } from '~/models/service';

export type ResourcesPayload = {
  memory_bytes: number | null;
  nano_cpus: number | null;
  pids_limit: number | null;
  shm_size: number | null;
};

export type ServiceKindMeta = {
  name: ServiceKind;
  supported_versions: string[];
  default_env_vars: Record<string, string>;
};

type ProvisionServicePayload = {
  appSlug: string;
  kind: ServiceKind;
  name: string;
  version: string;
  envVars: Record<string, string>;
  resources?: ResourcesPayload | null;
};

type ServiceRef = { appSlug: string; serviceId: string };

type UpdateServiceEnvVarsPayload = {
  appSlug: string;
  serviceId: string;
  vars: Record<string, string>;
};

export function getServiceKindsOptions() {
  return queryOptions({
    queryKey: ['services', 'kinds'],
    queryFn: () => httpGet<{ kinds: ServiceKindMeta[] }>('services/kinds'),
  });
}

export function getAppServicesOptions(appSlug: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services'],
    queryFn: () => httpGet<{ services: Service[] }>(`apps/${appSlug}/services`),
  });
}

export function getServiceOptions(appSlug: string, serviceId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services', serviceId],
    queryFn: () =>
      httpGet<{ service: Service }>(`apps/${appSlug}/services/${serviceId}`),
  });
}

export type ServiceStats = {
  running: boolean;
  started_at: string | null;
  cpu_percent: number | null;
  memory_used_bytes: number | null;
  disk_bytes: number | null;
};

export function getServiceStatsOptions(appSlug: string, serviceId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services', serviceId, 'stats'],
    queryFn: () =>
      httpGet<ServiceStats>(`apps/${appSlug}/services/${serviceId}/stats`),
  });
}

// Service lifecycle mutations all refresh the app's service list (which, by
// prefix, also covers the single-service and stats queries).
function useInvalidateServices() {
  const queryClient = useQueryClient();
  return (appSlug: string) =>
    queryClient.invalidateQueries({
      queryKey: ['apps', appSlug, 'services'],
    });
}

export function useProvisionService() {
  const invalidate = useInvalidateServices();
  return useMutation({
    mutationFn: (data: ProvisionServicePayload) =>
      httpPost<{ service: Service }>(`apps/${data.appSlug}/services`, {
        kind: data.kind,
        name: data.name,
        version: data.version,
        env_vars: data.envVars,
        resources: data.resources ?? null,
      }),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useRestartService() {
  const invalidate = useInvalidateServices();
  return useMutation({
    mutationFn: (data: ServiceRef) =>
      httpPost<{ restarted: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/restart`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useRedeployService() {
  const invalidate = useInvalidateServices();
  return useMutation({
    mutationFn: (data: ServiceRef) =>
      httpPost<{ redeploying: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/redeploy`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useStopService() {
  const invalidate = useInvalidateServices();
  return useMutation({
    mutationFn: (data: ServiceRef) =>
      httpPost<{ stopped: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/stop`,
        {}
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function useDeleteService() {
  const invalidate = useInvalidateServices();
  return useMutation({
    mutationFn: (data: ServiceRef) =>
      httpDelete<{ deleted: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}`
      ),
    onSuccess: (_, variables) => invalidate(variables.appSlug),
  });
}

export function getServiceEnvVarsOptions(appSlug: string, serviceId: string) {
  return queryOptions({
    queryKey: ['apps', appSlug, 'services', serviceId, 'env-vars'],
    queryFn: () =>
      httpGet<{ env_vars: Record<string, string> }>(
        `apps/${appSlug}/services/${serviceId}/env`
      ),
  });
}

export function useUpdateServiceEnvVars() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateServiceEnvVarsPayload) =>
      httpPut<{ env_vars: Record<string, string> }>(
        `apps/${data.appSlug}/services/${data.serviceId}/env`,
        { vars: data.vars }
      ),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: [
          'apps',
          variables.appSlug,
          'services',
          variables.serviceId,
          'env-vars',
        ],
      });
    },
  });
}
