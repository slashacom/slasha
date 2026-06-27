import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import type { AlertRule } from '~/models/alert_rule';
import type { Channel } from '~/models/channel';

export type ChannelInput = {
  name: string;
  kind: string;
  config: Record<string, unknown>;
};

export type RuleInput = {
  name: string;
  enabled: boolean;
  target: string;
  event: string;
  params: Record<string, unknown>;
  cooldown_secs: number;
  action_type: string;
  action_config: Record<string, unknown>;
};

export function ruleToInput(rule: AlertRule): RuleInput {
  const parse = (raw: string): Record<string, unknown> => {
    try {
      return JSON.parse(raw) as Record<string, unknown>;
    } catch {
      return {};
    }
  };
  return {
    name: rule.name,
    enabled: rule.enabled,
    target: rule.target,
    event: rule.event,
    params: parse(rule.params),
    cooldown_secs: rule.cooldown_secs,
    action_type: rule.action_type,
    action_config: parse(rule.action_config),
  };
}

export function getChannelsOptions() {
  return queryOptions({
    queryKey: ['channels'],
    queryFn: () => httpGet<Channel[]>('channels'),
  });
}

export function getAlertRulesOptions() {
  return queryOptions({
    queryKey: ['alert-rules'],
    queryFn: () => httpGet<AlertRule[]>('alert-rules'),
  });
}

export function useCreateChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: ChannelInput) => httpPost<Channel>('channels', input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channels'] });
    },
  });
}

export function useUpdateChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; input: Omit<ChannelInput, 'kind'> }) =>
      httpPut<Channel>(`channels/${vars.id}`, vars.input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channels'] });
    },
  });
}

export function useDeleteChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => httpDelete(`channels/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channels'] });
    },
  });
}

export function useTestChannel() {
  return useMutation({
    mutationFn: (id: string) => httpPost(`channels/${id}/test`, {}),
  });
}

export function useCreateRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: RuleInput) => httpPost<AlertRule>('alert-rules', input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
    },
  });
}

export function useUpdateRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; input: RuleInput }) =>
      httpPut<AlertRule>(`alert-rules/${vars.id}`, vars.input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
    },
  });
}

export function useDeleteRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => httpDelete(`alert-rules/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
    },
  });
}
