import type {
  AlertChannel,
  AlertChannelConfig,
  AlertRule,
  AlertRuleConfig,
} from '~/models/alerts';
import type { App } from '~/models/app';

export type ChannelDraft = {
  id: string | null;
  name: string;
  kind: AlertChannelConfig['kind'];
  webhook_url: string;
  bot_token: string;
  chat_id: string;
  smtp_host: string;
  smtp_port: string;
  smtp_username: string;
  smtp_password: string;
  from_address: string;
  to_address: string;
  enabled: boolean;
};

export type RuleDraft = {
  id: string | null;
  name: string;
  kind: AlertRuleConfig['kind'];
  threshold_percent: string;
  threshold: string;
  app_id: string;
  domain: string;
  days_before: string;
  health_check_url: string;
  cron_job_id: string;
  channel_ids: string[];
  direct_webhook_url: string;
  message_template: string;
  shell_command: string;
  enabled: boolean;
  cooldown_secs: string;
};

export const DEFAULT_ALERT_COOLDOWN_SECS = 900;

type ChannelKind = AlertChannelConfig['kind'];
type RuleKind = AlertRuleConfig['kind'];

type ChannelDefinition<K extends ChannelKind> = {
  label: string;
  description: string;
  buildConfig: (
    draft: ChannelDraft
  ) => Extract<AlertChannelConfig, { kind: K }>;
  summary: (config: Extract<AlertChannelConfig, { kind: K }>) => string;
};

type RuleDefinition<K extends RuleKind> = {
  label: string;
  description: string;
  defaults: Partial<RuleDraft>;
  buildConfig: (draft: RuleDraft) => Extract<AlertRuleConfig, { kind: K }>;
  summary: (
    config: Extract<AlertRuleConfig, { kind: K }>,
    apps: App[]
  ) => string;
};

export const alertChannelRegistry = {
  slack: {
    label: 'Slack',
    description: 'Send the alert to a Slack webhook.',
    buildConfig: (draft) => ({
      kind: 'slack',
      webhook_url: draft.webhook_url,
    }),
    summary: () => 'Slack webhook',
  },
  discord: {
    label: 'Discord',
    description: 'Send the alert to a Discord webhook.',
    buildConfig: (draft) => ({
      kind: 'discord',
      webhook_url: draft.webhook_url,
    }),
    summary: () => 'Discord webhook',
  },
  telegram: {
    label: 'Telegram',
    description: 'Send the alert to a Telegram chat.',
    buildConfig: (draft) => ({
      kind: 'telegram',
      bot_token: draft.bot_token,
      chat_id: draft.chat_id,
    }),
    summary: (config) => `Telegram chat ${config.chat_id}`,
  },
  email: {
    label: 'Email',
    description: 'Send the alert to an email address using Resend.',
    buildConfig: (draft) => {
      const smtp_port = parseInt(draft.smtp_port || '587', 10);
      return {
        kind: 'email',
        smtp_host: draft.smtp_host || '',
        smtp_port: isNaN(smtp_port) ? 587 : smtp_port,
        smtp_username: draft.smtp_username || '',
        smtp_password: draft.smtp_password || '',
        from_address: draft.from_address || '',
        to_address: draft.to_address || '',
      };
    },
    summary: (config) => `Email to ${config.to_address}`,
  },
} satisfies { [K in ChannelKind]: ChannelDefinition<K> };

export const alertRuleRegistry = {
  server_cpu: {
    label: 'Server CPU',
    description: 'Trigger when server CPU usage crosses a threshold.',
    defaults: { threshold_percent: '80' },
    buildConfig: (draft) => ({
      kind: 'server_cpu',
      threshold_percent: Number(draft.threshold_percent) || 0,
    }),
    summary: (config) => `Server CPU >= ${config.threshold_percent}%`,
  },
  server_memory: {
    label: 'Server Memory',
    description: 'Trigger when server memory usage crosses a threshold.',
    defaults: { threshold_percent: '80' },
    buildConfig: (draft) => ({
      kind: 'server_memory',
      threshold_percent: Number(draft.threshold_percent) || 0,
    }),
    summary: (config) => `Server memory >= ${config.threshold_percent}%`,
  },
  server_load_average: {
    label: 'Server Load Average',
    description: 'Trigger when load average crosses a threshold.',
    defaults: { threshold: '2' },
    buildConfig: (draft) => ({
      kind: 'server_load_average',
      threshold: Number(draft.threshold) || 0,
    }),
    summary: (config) => `Load average >= ${config.threshold}`,
  },
  app_cpu: {
    label: 'App CPU',
    description: "Trigger when an application's CPU usage crosses a threshold.",
    defaults: { app_id: '', threshold_percent: '80' },
    buildConfig: (draft) => ({
      kind: 'app_cpu',
      app_id: draft.app_id,
      threshold_percent: Number(draft.threshold_percent) || 0,
    }),
    summary: (config, apps) =>
      `App CPU for ${appName(config.app_id, apps)} >= ${config.threshold_percent}%`,
  },
  app_memory: {
    label: 'App Memory',
    description:
      "Trigger when an application's memory usage crosses a threshold.",
    defaults: { app_id: '', threshold_percent: '80' },
    buildConfig: (draft) => ({
      kind: 'app_memory',
      app_id: draft.app_id,
      threshold_percent: Number(draft.threshold_percent) || 0,
    }),
    summary: (config, apps) =>
      `App memory for ${appName(config.app_id, apps)} >= ${config.threshold_percent}%`,
  },
  domain_tls_expiry: {
    label: 'Domain TLS Expiry',
    description: 'Trigger when a certificate is close to expiring.',
    defaults: { domain: '', days_before: '30' },
    buildConfig: (draft) => ({
      kind: 'domain_tls_expiry',
      domain: draft.domain,
      days_before: Number(draft.days_before) || 0,
    }),
    summary: (config) =>
      `TLS cert for ${config.domain} expires in ${config.days_before} days`,
  },
  domain_dns_misconfigured: {
    label: 'Domain DNS Misconfigured',
    description: 'Trigger when a domain resolves incorrectly or not at all.',
    defaults: { domain: '' },
    buildConfig: (draft) => ({
      kind: 'domain_dns_misconfigured',
      domain: draft.domain,
    }),
    summary: (config) => `DNS misconfiguration for ${config.domain}`,
  },
  app_health_check: {
    label: 'App Health Check',
    description:
      'Trigger when an application health check URL does not return a 2xx response.',
    defaults: { app_id: '', health_check_url: '' },
    buildConfig: (draft) => ({
      kind: 'app_health_check',
      app_id: draft.app_id,
      health_check_url: draft.health_check_url,
    }),
    summary: (config, apps) =>
      `Health check for ${appName(config.app_id, apps)}: ${config.health_check_url}`,
  },
  cron_failed: {
    label: 'Cron Failed',
    description: 'Trigger when the most recent run of a cron job fails.',
    defaults: { cron_job_id: '' },
    buildConfig: (draft) => ({
      kind: 'cron_failed',
      cron_job_id: draft.cron_job_id,
    }),
    summary: () => 'Latest run failed',
  },
} satisfies { [K in RuleKind]: RuleDefinition<K> };

export const alertChannelKinds = Object.keys(
  alertChannelRegistry
) as ChannelKind[];
export const alertRuleKinds = Object.keys(alertRuleRegistry) as RuleKind[];

export function emptyChannelDraft(
  kind: ChannelDraft['kind'] = 'slack'
): ChannelDraft {
  return {
    id: null,
    name: '',
    kind,
    webhook_url: '',
    bot_token: '',
    chat_id: '',
    smtp_host: '',
    smtp_port: '587',
    smtp_username: '',
    smtp_password: '',
    from_address: '',
    to_address: '',
    enabled: true,
  };
}

export function channelDraftFromChannel(channel: AlertChannel): ChannelDraft {
  const draft = emptyChannelDraft(channel.config.kind);
  draft.id = channel.id;
  draft.name = channel.name;
  draft.enabled = channel.enabled;

  for (const [key, value] of Object.entries(channel.config)) {
    if (key !== 'kind' && value != null) {
      (draft as any)[key] = String(value);
    }
  }

  return draft;
}

export function emptyRuleDraft(
  kind: AlertRuleConfig['kind'] = 'server_cpu',
  cooldownSecs = 900
): RuleDraft {
  return {
    id: null,
    name: '',
    kind,
    threshold_percent: '80',
    threshold: '2',
    app_id: '',
    domain: '',
    days_before: '30',
    health_check_url: '',
    cron_job_id: '',
    channel_ids: [],
    direct_webhook_url: '',
    message_template: '',
    shell_command: '',
    enabled: true,
    cooldown_secs: String(cooldownSecs),
    ...alertRuleRegistry[kind].defaults,
  };
}

export function ruleDraftFromRule(
  rule: AlertRule,
  cooldownSecs = rule.cooldown_secs
): RuleDraft {
  const draft = emptyRuleDraft(rule.config.kind, cooldownSecs);
  draft.id = rule.id;
  draft.name = rule.name;
  draft.channel_ids = [...rule.channel_ids];
  draft.direct_webhook_url = rule.direct_webhook_url ?? '';
  draft.message_template = rule.message_template ?? '';
  draft.shell_command = rule.shell_command ?? '';
  draft.enabled = rule.enabled;
  draft.cooldown_secs = String(rule.cooldown_secs);

  for (const [key, value] of Object.entries(rule.config)) {
    if (key !== 'kind' && value != null) {
      (draft as any)[key] = String(value);
    }
  }

  return draft;
}

export function buildChannelConfig(draft: ChannelDraft) {
  return alertChannelRegistry[draft.kind].buildConfig(
    draft
  ) as AlertChannelConfig;
}

export function buildRuleConfig(draft: RuleDraft) {
  return alertRuleRegistry[draft.kind].buildConfig(draft) as AlertRuleConfig;
}

export function channelSummary(channel: AlertChannel) {
  return alertChannelRegistry[channel.config.kind].summary(
    channel.config as never
  );
}

export function configSummary(rule: AlertRule, apps: App[]) {
  return alertRuleRegistry[rule.config.kind].summary(
    rule.config as never,
    apps
  );
}

export function deliverySummary(
  rule: AlertRule,
  channels: Map<string, { name: string }>
) {
  const targets = rule.channel_ids.map(
    (id) => channels.get(id)?.name ?? 'Unknown channel'
  );
  if (rule.direct_webhook_url) {
    targets.push('Webhook');
  }
  if (rule.shell_command) {
    targets.push('Shell');
  }
  return targets.length > 0 ? targets.join(', ') : 'No delivery target';
}

function appName(appId: string, apps: App[]) {
  const app = apps.find((item) => item.id === appId);
  return app?.name ?? app?.slug ?? appId;
}
