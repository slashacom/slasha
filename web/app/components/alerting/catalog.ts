import type { AlertRule } from '~/models/alert_rule';
import type { Channel } from '~/models/channel';

export type EventScope = 'server' | 'app' | 'domain';
export type Comparator = 'gt' | 'lt';

export type AlertEventDef = {
  value: string;
  label: string;
  scope: EventScope;
  comparator: Comparator;
  unit: string;
  defaultThreshold: number;
  description: string;
  boolean?: boolean;
};

export const ALERT_EVENTS: AlertEventDef[] = [
  {
    value: 'server.cpu',
    label: 'Server CPU',
    scope: 'server',
    comparator: 'gt',
    unit: '%',
    defaultThreshold: 80,
    description: 'Host CPU usage rises above the threshold',
  },
  {
    value: 'server.memory',
    label: 'Server memory',
    scope: 'server',
    comparator: 'gt',
    unit: '%',
    defaultThreshold: 90,
    description: 'Host memory usage rises above the threshold',
  },
  {
    value: 'server.disk',
    label: 'Server disk',
    scope: 'server',
    comparator: 'gt',
    unit: '%',
    defaultThreshold: 85,
    description: 'Root filesystem usage rises above the threshold',
  },
  {
    value: 'server.load',
    label: 'Server load average',
    scope: 'server',
    comparator: 'gt',
    unit: '',
    defaultThreshold: 4,
    description: '1-minute load average rises above the threshold',
  },
  {
    value: 'app.cpu',
    label: 'App CPU',
    scope: 'app',
    comparator: 'gt',
    unit: '%',
    defaultThreshold: 90,
    description: "An app's containers exceed this CPU usage",
  },
  {
    value: 'app.memory',
    label: 'App memory',
    scope: 'app',
    comparator: 'gt',
    unit: '%',
    defaultThreshold: 90,
    description: "An app's memory usage approaches its limit",
  },
  {
    value: 'domain.cert_days',
    label: 'TLS certificate expiring',
    scope: 'domain',
    comparator: 'lt',
    unit: 'days',
    defaultThreshold: 14,
    description: 'A domain certificate expires within this many days',
  },
  {
    value: 'domain.dns_problem',
    label: 'DNS misconfigured',
    scope: 'domain',
    comparator: 'gt',
    unit: '',
    defaultThreshold: 0.5,
    boolean: true,
    description: 'A domain stops resolving to this server',
  },
];

export function eventDef(value: string): AlertEventDef | undefined {
  return ALERT_EVENTS.find((event) => event.value === value);
}

export type ActionType = 'channel' | 'webhook' | 'execute_program';

export const ACTION_TYPES: { value: ActionType; label: string }[] = [
  { value: 'channel', label: 'Send to a channel' },
  { value: 'webhook', label: 'Call a webhook URL' },
  { value: 'execute_program', label: 'Run a command' },
];

export type ChannelKind = 'slack' | 'telegram';

export const CHANNEL_KINDS: { value: ChannelKind; label: string }[] = [
  { value: 'slack', label: 'Slack' },
  { value: 'telegram', label: 'Telegram' },
];

export function parseJson<T>(raw: string): T {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return {} as T;
  }
}

export function ruleThreshold(rule: AlertRule): number | null {
  const params = parseJson<{ gt?: number; lt?: number }>(rule.params);
  const def = eventDef(rule.event);
  if (!def) {
    return params.gt ?? params.lt ?? null;
  }
  return params[def.comparator] ?? null;
}

export function channelLabel(channel: Channel): string {
  const kind = CHANNEL_KINDS.find((entry) => entry.value === channel.kind);
  return kind ? kind.label : channel.kind;
}
