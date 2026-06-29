import type {
  AlertChannel,
  AlertChannelConfig,
  AlertRule,
  AlertRuleConfig,
} from '~/models/alerts';

export type ChannelDraft = {
  id: string | null;
  name: string;
  kind: AlertChannelConfig['kind'];
  webhook_url: string;
  bot_token: string;
  chat_id: string;
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
  channel_ids: string[];
  direct_webhook_url: string;
  message_template: string;
  shell_command: string;
  enabled: boolean;
  cooldown_secs: string;
};

export const DEFAULT_ALERT_COOLDOWN_SECS = 900;

type BuildResult<T> =
  | { config: T; error?: never }
  | { config?: never; error: string };

type AppSummary = { id: string; name: string; slug: string };
type ChannelKind = AlertChannelConfig['kind'];
type RuleKind = AlertRuleConfig['kind'];

type ChannelDefinition<K extends ChannelKind> = {
  label: string;
  description: string;
  defaults: Partial<ChannelDraft>;
  buildConfig: (
    draft: ChannelDraft
  ) => BuildResult<Extract<AlertChannelConfig, { kind: K }>>;
  summary: (config: Extract<AlertChannelConfig, { kind: K }>) => string;
};

type RuleDefinition<K extends RuleKind> = {
  label: string;
  description: string;
  defaults: Partial<RuleDraft>;
  buildConfig: (
    draft: RuleDraft
  ) => BuildResult<Extract<AlertRuleConfig, { kind: K }>>;
  summary: (
    config: Extract<AlertRuleConfig, { kind: K }>,
    apps: AppSummary[]
  ) => string;
};

export const alertChannelRegistry = {
  slack: {
    label: 'Slack',
    description: 'Send the alert to a Slack webhook.',
    defaults: { webhook_url: '' },
    buildConfig: (draft) => {
      const webhook_url = draft.webhook_url.trim();
      return webhook_url
        ? { config: { kind: 'slack', webhook_url } }
        : { error: 'Slack webhook URL is required.' };
    },
    summary: () => 'Slack webhook',
  },
  telegram: {
    label: 'Telegram',
    description: 'Send the alert to a Telegram chat.',
    defaults: { bot_token: '', chat_id: '' },
    buildConfig: (draft) => {
      const bot_token = draft.bot_token.trim();
      const chat_id = draft.chat_id.trim();
      if (!bot_token) return { error: 'Telegram bot token is required.' };
      if (!chat_id) return { error: 'Telegram chat id is required.' };
      return { config: { kind: 'telegram', bot_token, chat_id } };
    },
    summary: (config) => `Telegram chat ${config.chat_id}`,
  },
} satisfies { [K in ChannelKind]: ChannelDefinition<K> };

export const alertRuleRegistry = {
  server_cpu: {
    label: 'Server CPU',
    description: 'Trigger when server CPU usage crosses a threshold.',
    defaults: { threshold_percent: '80' },
    buildConfig: (draft) =>
      numberConfig(
        draft.threshold_percent,
        'Threshold must be a number.',
        (threshold_percent) => ({
          kind: 'server_cpu',
          threshold_percent,
        })
      ),
    summary: (config) => `Server CPU >= ${config.threshold_percent}%`,
  },
  server_memory: {
    label: 'Server Memory',
    description: 'Trigger when server memory usage crosses a threshold.',
    defaults: { threshold_percent: '80' },
    buildConfig: (draft) =>
      numberConfig(
        draft.threshold_percent,
        'Threshold must be a number.',
        (threshold_percent) => ({
          kind: 'server_memory',
          threshold_percent,
        })
      ),
    summary: (config) => `Server memory >= ${config.threshold_percent}%`,
  },
  server_load_average: {
    label: 'Server Load Average',
    description: 'Trigger when load average crosses a threshold.',
    defaults: { threshold: '2' },
    buildConfig: (draft) =>
      numberConfig(
        draft.threshold,
        'Threshold must be a number.',
        (threshold) => ({
          kind: 'server_load_average',
          threshold,
        })
      ),
    summary: (config) => `Load average >= ${config.threshold}`,
  },
  app_cpu: {
    label: 'App CPU',
    description: "Trigger when an application's CPU usage crosses a threshold.",
    defaults: { app_id: '', threshold_percent: '80' },
    buildConfig: (draft) =>
      buildAppConfig(draft, (app_id, threshold_percent) => ({
        kind: 'app_cpu',
        app_id,
        threshold_percent,
      })),
    summary: (config, apps) =>
      `App CPU for ${appName(config.app_id, apps)} >= ${config.threshold_percent}%`,
  },
  app_memory: {
    label: 'App Memory',
    description:
      "Trigger when an application's memory usage crosses a threshold.",
    defaults: { app_id: '', threshold_percent: '80' },
    buildConfig: (draft) =>
      buildAppConfig(draft, (app_id, threshold_percent) => ({
        kind: 'app_memory',
        app_id,
        threshold_percent,
      })),
    summary: (config, apps) =>
      `App memory for ${appName(config.app_id, apps)} >= ${config.threshold_percent}%`,
  },
  domain_tls_expiry: {
    label: 'Domain TLS Expiry',
    description: 'Trigger when a certificate is close to expiring.',
    defaults: { domain: '', days_before: '30' },
    buildConfig: (draft) => {
      const domain = draft.domain.trim();
      if (!domain) return { error: 'Domain is required.' };
      return numberConfig(
        draft.days_before,
        'Days before expiry must be a number.',
        (days_before) => ({
          kind: 'domain_tls_expiry',
          domain,
          days_before,
        })
      );
    },
    summary: (config) =>
      `TLS cert for ${config.domain} expires in ${config.days_before} days`,
  },
  domain_dns_misconfigured: {
    label: 'Domain DNS Misconfigured',
    description: 'Trigger when a domain resolves incorrectly or not at all.',
    defaults: { domain: '' },
    buildConfig: (draft) => {
      const domain = draft.domain.trim();
      return domain
        ? { config: { kind: 'domain_dns_misconfigured', domain } }
        : { error: 'Domain is required.' };
    },
    summary: (config) => `DNS misconfiguration for ${config.domain}`,
  },
  app_health_check: {
    label: 'App Health Check',
    description:
      'Trigger when an application health check URL does not return a 2xx response.',
    defaults: { app_id: '', health_check_url: '' },
    buildConfig: (draft) => {
      const app_id = draft.app_id.trim();
      if (!app_id) return { error: 'Select an app for the rule.' };

      const url = draft.health_check_url.trim();
      if (!url) return { error: 'Health check URL is required.' };

      try {
        const parsed = new URL(url);
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
          return { error: 'Health check URL must use http:// or https://' };
        }
      } catch {
        return { error: 'Health check URL is invalid.' };
      }

      return { config: { kind: 'app_health_check', app_id, url } };
    },
    summary: (config, apps) =>
      `Health check for ${appName(config.app_id, apps)}: ${config.url}`,
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
    enabled: true,
    ...alertChannelRegistry[kind].defaults,
  };
}

export function channelDraftFromChannel(channel: AlertChannel): ChannelDraft {
  const isSlack = channel.config.kind === 'slack';
  return {
    id: channel.id,
    name: channel.name,
    kind: channel.config.kind,
    webhook_url: isSlack
      ? (channel.config as Extract<AlertChannelConfig, { kind: 'slack' }>)
          .webhook_url
      : '',
    bot_token: !isSlack
      ? (channel.config as Extract<AlertChannelConfig, { kind: 'telegram' }>)
          .bot_token
      : '',
    chat_id: !isSlack
      ? (channel.config as Extract<AlertChannelConfig, { kind: 'telegram' }>)
          .chat_id
      : '',
    enabled: channel.enabled,
  };
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

  const cfg = rule.config;
  switch (cfg.kind) {
    case 'server_cpu':
    case 'server_memory':
      draft.threshold_percent = String(cfg.threshold_percent);
      break;
    case 'server_load_average':
      draft.threshold = String(cfg.threshold);
      break;
    case 'app_cpu':
    case 'app_memory':
      draft.app_id = cfg.app_id;
      draft.threshold_percent = String(cfg.threshold_percent);
      break;
    case 'domain_tls_expiry':
      draft.domain = cfg.domain;
      draft.days_before = String(cfg.days_before);
      break;
    case 'domain_dns_misconfigured':
      draft.domain = cfg.domain;
      break;
    case 'app_health_check':
      draft.app_id = cfg.app_id;
      draft.health_check_url = cfg.url;
      break;
  }

  return draft;
}

export function buildChannelConfig(draft: ChannelDraft) {
  return alertChannelRegistry[draft.kind].buildConfig(
    draft
  ) as BuildResult<AlertChannelConfig>;
}

export function buildRuleConfig(draft: RuleDraft) {
  return alertRuleRegistry[draft.kind].buildConfig(
    draft
  ) as BuildResult<AlertRuleConfig>;
}

export function channelSummary(channel: AlertChannel) {
  return alertChannelRegistry[channel.config.kind].summary(
    channel.config as never
  );
}

export function configSummary(rule: AlertRule, apps: AppSummary[]) {
  return alertRuleRegistry[rule.config.kind].summary(
    rule.config as never,
    apps
  );
}

export function parseNumber(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

function numberConfig<T>(
  value: string,
  error: string,
  create: (value: number) => T
): BuildResult<T> {
  const parsed = parseNumber(value);
  return parsed === null ? { error } : { config: create(parsed) };
}

function buildAppConfig<T>(
  draft: RuleDraft,
  create: (appId: string, threshold: number) => T
): BuildResult<T> {
  const appId = draft.app_id.trim();
  if (!appId) return { error: 'Select an app for the rule.' };
  return numberConfig(
    draft.threshold_percent,
    'Threshold must be a number.',
    (threshold) => create(appId, threshold)
  );
}

function appName(appId: string, apps: AppSummary[]) {
  const app = apps.find((item) => item.id === appId);
  return app?.name ?? app?.slug ?? appId;
}
