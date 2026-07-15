import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { Node } from '~/models/node';
import type { ServerMetrics } from '~/models/server-metrics';

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

export function getNodeMetricsOptions(
  nodeId: string,
  start?: Date,
  end?: Date
) {
  let queryParams = new URLSearchParams();
  if (start) queryParams.append('start', start.toISOString());
  if (end) queryParams.append('end', end.toISOString());

  const qs = queryParams.toString();

  return queryOptions({
    queryKey: ['nodes', nodeId, 'metrics', { start, end }],
    queryFn: () =>
      httpGet<{ metrics: ServerMetrics[] }>(
        `nodes/${nodeId}/metrics${qs ? `?${qs}` : ''}`
      ),
  });
}
