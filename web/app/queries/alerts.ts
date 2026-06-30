import { queryOptions, useMutation } from '@tanstack/react-query';
import { httpDelete, httpGet, httpPost, httpPut } from '~/utils/http';
import { queryClient } from '~/utils/query-client';
import type {
  AlertChannel,
  AlertChannelConfig,
  AlertIncident,
  AlertNotification,
  AlertRule,
  AlertRuleConfig,
} from '~/models/alerts';
import type { CronJob } from '~/models/cron';
type AlertChannelPayload = {
  name: string;
  config: AlertChannelConfig;
  enabled: boolean;
};

type AlertRulePayload = {
  name: string;
  config: AlertRuleConfig;
  channel_ids: string[];
  direct_webhook_url?: string | null;
  message_template?: string | null;
  shell_command?: string | null;
  enabled: boolean;
  cooldown_secs?: number;
};

export function getAllCronsOptions() {
  return queryOptions({
    queryKey: ['alerts', 'crons'],
    queryFn: () => httpGet<{ crons: CronJob[] }>('alerts/crons'),
  });
}

export function getAlertChannelsOptions() {
  return queryOptions({
    queryKey: ['alerts', 'channels'],
    queryFn: () => httpGet<{ channels: AlertChannel[] }>('alerts/channels'),
  });
}

export function getAlertRulesOptions() {
  return queryOptions({
    queryKey: ['alerts', 'rules'],
    queryFn: () => httpGet<{ rules: AlertRule[] }>('alerts/rules'),
  });
}

export function getAlertIncidentsOptions() {
  return queryOptions({
    queryKey: ['alerts', 'incidents'],
    queryFn: () => httpGet<{ incidents: AlertIncident[] }>('alerts/incidents'),
  });
}

export function getAlertNotificationsOptions() {
  return queryOptions({
    queryKey: ['alerts', 'notifications'],
    queryFn: () =>
      httpGet<{ notifications: AlertNotification[] }>('alerts/notifications'),
  });
}

export function getAlertIncidentNotificationsOptions(id: string) {
  return queryOptions({
    queryKey: ['alerts', 'incidents', id, 'notifications'],
    queryFn: () =>
      httpGet<{
        incident: AlertIncident;
        notifications: AlertNotification[];
      }>(`alerts/incidents/${id}/notifications`),
  });
}

export function useCreateAlertChannel() {
  return useMutation({
    mutationFn: (payload: AlertChannelPayload) =>
      httpPost<{ channel: AlertChannel }>('alerts/channels', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'channels'] });
    },
  });
}

export function useUpdateAlertChannel() {
  return useMutation({
    mutationFn: (payload: { id: string; data: AlertChannelPayload }) =>
      httpPut<{ channel: AlertChannel }>(
        `alerts/channels/${payload.id}`,
        payload.data
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'channels'] });
    },
  });
}

export function useDeleteAlertChannel() {
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ deleted: boolean }>(`alerts/channels/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'channels'] });
      queryClient.invalidateQueries({ queryKey: ['alerts', 'rules'] });
    },
  });
}

export function useCreateAlertRule() {
  return useMutation({
    mutationFn: (payload: AlertRulePayload) =>
      httpPost<{ rule: AlertRule }>('alerts/rules', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'rules'] });
    },
  });
}

export function useUpdateAlertRule() {
  return useMutation({
    mutationFn: (payload: { id: string; data: AlertRulePayload }) =>
      httpPut<{ rule: AlertRule }>(`alerts/rules/${payload.id}`, payload.data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'rules'] });
    },
  });
}

export function useDeleteAlertRule() {
  return useMutation({
    mutationFn: (id: string) =>
      httpDelete<{ deleted: boolean }>(`alerts/rules/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts', 'rules'] });
    },
  });
}
