import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { Node } from '~/models/node';
import type { NodeMetrics } from '~/models/node-metrics';

export type NodeWithInfo = Node & {
  live_status: string;
  os?: string;
};

export type CreateNodePayload = {
  name: string;
  host: string;
  user: string;
  port?: number;
  ssh_private_key: string;
};

export type UpdateNodePayload = {
  name?: string;
  host?: string;
  user?: string;
  port?: number;
  ssh_private_key?: string;
};

export function getNodesOptions() {
  return queryOptions({
    queryKey: ['nodes'],
    queryFn: () => httpGet<{ nodes: NodeWithInfo[] }>('nodes'),
  });
}

export function getNodeOptions(id: string) {
  return queryOptions({
    queryKey: ['nodes', id],
    queryFn: () => httpGet<{ node: NodeWithInfo }>(`nodes/${id}`),
  });
}

export function useCreateNode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateNodePayload) =>
      httpPost<{ node: Node }>('nodes', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['nodes'] });
    },
  });
}

export function useUpdateNode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: UpdateNodePayload }) =>
      httpPut<{ node: Node }>(`nodes/${id}`, payload),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['nodes', variables.id] });
      queryClient.invalidateQueries({ queryKey: ['nodes'] });
    },
  });
}

export function useDeleteNode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ deleting: boolean; deleted: boolean }>(`nodes/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['nodes'] });
    },
  });
}

const REFRESH_INTERVAL = 15000;

export function getNodeMetricsOptions(nodeId: string, hours: number) {
  return queryOptions({
    queryKey: ['nodes', nodeId, 'metrics', hours],
    queryFn: () => {
      const end = new Date();
      const start = new Date(end.getTime() - hours * 3600 * 1000);
      const queryParams = new URLSearchParams({
        start: start.toISOString(),
        end: end.toISOString(),
      });
      return httpGet<{ metrics: NodeMetrics[] }>(
        `nodes/${nodeId}/metrics?${queryParams.toString()}`
      );
    },
    staleTime: REFRESH_INTERVAL,
    refetchInterval: REFRESH_INTERVAL,
  });
}

export function getLatestNodeMetricOptions(nodeId: string) {
  return queryOptions({
    queryKey: ['nodes', nodeId, 'metrics', 'latest'],
    queryFn: () =>
      httpGet<{ metric: NodeMetrics | null }>(`nodes/${nodeId}/metrics/latest`),
    staleTime: REFRESH_INTERVAL,
    refetchInterval: REFRESH_INTERVAL,
  });
}
