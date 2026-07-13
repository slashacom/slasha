import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { Node } from '~/models/node';

export type NodeWithStatus = Node & {
  live_status: string;
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
    queryFn: () => httpGet<{ nodes: NodeWithStatus[] }>('nodes'),
  });
}

export function getNodeOptions(id: string) {
  return queryOptions({
    queryKey: ['nodes', id],
    queryFn: () => httpGet<{ node: NodeWithStatus }>(`nodes/${id}`),
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

export function useUpdateNode(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateNodePayload) =>
      httpPut<{ node: Node }>(`nodes/${id}`, payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['nodes', id] });
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
