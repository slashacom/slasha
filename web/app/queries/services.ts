import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpGet, httpPost, httpDelete, httpPut } from '~/utils/http';
import type { Service, ServiceKind } from '~/models/service';

export type ResourcesPayload = {
  memory_bytes: number | null;
  nano_cpus: number | null;
  pids_limit: number | null;
  shm_size: number | null;
}

export type ServiceKindMeta = {
  name: ServiceKind;
  supported_versions: string[];
  default_env_vars: Record<string, string>;
}

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

export function useProvisionService() {
  return useMutation({
    mutationFn: (data: {
      appSlug: string;
      kind: ServiceKind;
      name: string;
      version: string;
      envVars: Record<string, string>;
      resources?: ResourcesPayload | null;
    }) =>
      httpPost<{ service: Service }>(`apps/${data.appSlug}/services`, {
        kind: data.kind,
        name: data.name,
        version: data.version,
        env_vars: data.envVars,
        resources: data.resources ?? null,
      }),
  });
}

export function useRestartService() {
  return useMutation({
    mutationFn: (data: { appSlug: string; serviceId: string }) =>
      httpPost<{ restarted: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/restart`,
        {}
      ),
  });
}

export function useRedeployService() {
  return useMutation({
    mutationFn: (data: { appSlug: string; serviceId: string }) =>
      httpPost<{ redeploying: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/redeploy`,
        {}
      ),
  });
}

export function useStopService() {
  return useMutation({
    mutationFn: (data: { appSlug: string; serviceId: string }) =>
      httpPost<{ stopped: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}/stop`,
        {}
      ),
  });
}

export function useDeleteService() {
  return useMutation({
    mutationFn: (data: { appSlug: string; serviceId: string }) =>
      httpDelete<{ deleted: boolean }>(
        `apps/${data.appSlug}/services/${data.serviceId}`
      ),
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
  return useMutation({
    mutationFn: (data: {
      appSlug: string;
      serviceId: string;
      vars: Record<string, string>;
    }) =>
      httpPut<{ env_vars: Record<string, string> }>(
        `apps/${data.appSlug}/services/${data.serviceId}/env`,
        { vars: data.vars }
      ),
  });
}
