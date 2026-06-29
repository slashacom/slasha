import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Select } from '~/components/interface/select';
import { Switch } from '~/components/interface/switch';
import { Textarea } from '~/components/interface/textarea';
import type { AlertChannel, AlertRule, AlertRuleConfig } from '~/models/alerts';
import type { App } from '~/models/app';
import { useCreateAlertRule, useUpdateAlertRule } from '~/queries/alerts';
import {
  alertRuleKinds,
  alertRuleRegistry,
  buildRuleConfig,
  DEFAULT_ALERT_COOLDOWN_SECS,
  emptyRuleDraft,
  ruleDraftFromRule,
} from './alert-definitions';
import { ChannelMultiSelect } from './alert-channel-multi-select';

type AlertRuleFormProps = {
  apps: App[];
  channels: AlertChannel[];
  rule?: AlertRule;
  onCancel: () => void;
  onSaved: () => void;
};

export function AlertRuleForm(props: AlertRuleFormProps) {
  const { apps, channels, rule, onCancel, onSaved } = props;
  const createRule = useCreateAlertRule();
  const updateRule = useUpdateAlertRule();
  const [draft, setDraft] = useState(() =>
    rule
      ? ruleDraftFromRule(rule)
      : emptyRuleDraft('server_cpu', DEFAULT_ALERT_COOLDOWN_SECS)
  );

  const handleSave = async () => {
    const name = draft.name.trim();
    if (!name) {
      toast.error('Rule name is required.');
      return;
    }

    const cooldownSecs = Number(draft.cooldown_secs);
    if (!cooldownSecs || cooldownSecs <= 0) {
      toast.error('Cooldown must be greater than zero.');
      return;
    }

    const result = buildRuleConfig(draft);
    if ('error' in result) {
      toast.error(result.error);
      return;
    }

    const payload = {
      name,
      config: result.config,
      channel_ids: draft.channel_ids,
      direct_webhook_url: draft.direct_webhook_url.trim() || undefined,
      message_template: draft.message_template.trim() || undefined,
      shell_command: draft.shell_command.trim() || undefined,
      enabled: draft.enabled,
      cooldown_secs: cooldownSecs,
    };
    const promise = rule
      ? updateRule.mutateAsync({ id: rule.id, data: payload })
      : createRule.mutateAsync(payload);

    toast.promise(promise, {
      loading: rule ? 'Updating rule...' : 'Creating rule...',
      success: rule ? 'Rule updated.' : 'Rule created.',
      error: (error) => error.message || 'Failed to save rule.',
    });

    try {
      await promise;
      onSaved();
    } catch {
      return;
    }
  };

  return (
    <div className="max-w-2xl space-y-8">
      <FormSection
        title="Trigger"
        description="Choose what Slasha should monitor and when it should alert."
      >
        <Field label="Name">
          <Input
            value={draft.name}
            onChange={(event) =>
              setDraft((current) => ({ ...current, name: event.target.value }))
            }
            placeholder="Server CPU warning"
          />
        </Field>

        <Field
          label="Alert kind"
          help={alertRuleRegistry[draft.kind].description}
        >
          <Select
            value={draft.kind}
            onChange={(event) => {
              const kind = event.target.value as AlertRuleConfig['kind'];
              setDraft((current) => ({
                ...emptyRuleDraft(
                  kind,
                  Number(current.cooldown_secs) || DEFAULT_ALERT_COOLDOWN_SECS
                ),
                id: current.id,
                name: current.name,
                channel_ids: current.channel_ids,
                direct_webhook_url: current.direct_webhook_url,
                message_template: current.message_template,
                shell_command: current.shell_command,
                enabled: current.enabled,
                cooldown_secs: current.cooldown_secs,
              }));
            }}
          >
            {alertRuleKinds.map((kind) => (
              <option key={kind} value={kind}>
                {alertRuleRegistry[kind].label}
              </option>
            ))}
          </Select>
        </Field>

        {draft.kind === 'server_cpu' || draft.kind === 'server_memory' ? (
          <NumberField
            label="Threshold percent"
            value={draft.threshold_percent}
            min={0}
            max={100}
            step={0.1}
            onChange={(threshold_percent) =>
              setDraft((current) => ({ ...current, threshold_percent }))
            }
          />
        ) : null}

        {draft.kind === 'server_load_average' ? (
          <NumberField
            label="Threshold"
            value={draft.threshold}
            min={0}
            step={0.1}
            onChange={(threshold) =>
              setDraft((current) => ({ ...current, threshold }))
            }
          />
        ) : null}

        {draft.kind === 'app_cpu' || draft.kind === 'app_memory' ? (
          <>
            <Field label="App">
              <Select
                value={draft.app_id}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    app_id: event.target.value,
                  }))
                }
              >
                <option value="">Select an app</option>
                {apps.map((app) => (
                  <option key={app.id} value={app.id}>
                    {app.name}
                  </option>
                ))}
              </Select>
            </Field>
            <NumberField
              label="Threshold percent"
              value={draft.threshold_percent}
              min={0}
              max={100}
              step={0.1}
              onChange={(threshold_percent) =>
                setDraft((current) => ({ ...current, threshold_percent }))
              }
            />
          </>
        ) : null}

        {draft.kind === 'domain_tls_expiry' ? (
          <>
            <Field label="Domain">
              <Input
                value={draft.domain}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    domain: event.target.value,
                  }))
                }
                placeholder="example.com"
              />
            </Field>
            <NumberField
              label="Days before expiry"
              value={draft.days_before}
              min={0}
              step={1}
              onChange={(days_before) =>
                setDraft((current) => ({ ...current, days_before }))
              }
            />
          </>
        ) : null}

        {draft.kind === 'domain_dns_misconfigured' ? (
          <Field label="Domain">
            <Input
              value={draft.domain}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  domain: event.target.value,
                }))
              }
              placeholder="example.com"
            />
          </Field>
        ) : null}
      </FormSection>

      <FormSection
        title="Delivery"
        description="Select reusable channels or configure an optional direct action."
      >
        <Field label="Channels">
          <ChannelMultiSelect
            channels={channels}
            selectedIds={draft.channel_ids}
            onChange={(channel_ids) =>
              setDraft((current) => ({ ...current, channel_ids }))
            }
          />
        </Field>

        <Field label="Direct webhook URL" help="Optional">
          <Input
            value={draft.direct_webhook_url}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                direct_webhook_url: event.target.value,
              }))
            }
            placeholder="https://..."
          />
        </Field>

        <Field label="Shell command" help="Optional">
          <Input
            value={draft.shell_command}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                shell_command: event.target.value,
              }))
            }
            placeholder="Command to run"
          />
          <ShellCommandEnvHelp />
        </Field>
      </FormSection>

      <FormSection
        title="Behavior"
        description="Control notification content and repeat frequency."
      >
        <Field label="Message template" help="Optional">
          <Textarea
            value={draft.message_template}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                message_template: event.target.value,
              }))
            }
            placeholder="Use {{value}}, {{detail}}, {{notification_status}}, {{alert_kind}}"
          />
          <TemplateVarHelp />
        </Field>

        <NumberField
          label="Cooldown seconds"
          value={draft.cooldown_secs}
          min={1}
          step={1}
          onChange={(cooldown_secs) =>
            setDraft((current) => ({ ...current, cooldown_secs }))
          }
        />

        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-semibold tracking-tight text-text">
              Enabled
            </p>
            <p className="text-[11px] text-text-tertiary">
              Disabled rules remain saved but are not evaluated.
            </p>
          </div>
          <Switch
            checked={draft.enabled}
            onCheckedChange={(enabled) =>
              setDraft((current) => ({ ...current, enabled }))
            }
          />
        </div>
      </FormSection>

      <div className="flex items-center gap-2 border-t border-border pt-6">
        <Button
          label={rule ? 'Save changes' : 'Create rule'}
          onClick={handleSave}
          isLoading={createRule.isPending || updateRule.isPending}
        />
        <Button label="Cancel" variant="ghost" onClick={onCancel} />
      </div>
    </div>
  );
}

function FormSection(props: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-5">
      <div>
        <h3 className="text-xs font-medium text-text-tertiary">
          {props.title}
        </h3>
        <p className="mt-1 text-[11px] text-text-tertiary">
          {props.description}
        </p>
      </div>
      {props.children}
    </section>
  );
}

function Field(props: {
  label: string;
  help?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <Label>{props.label}</Label>
        {props.help ? (
          <span className="text-[11px] text-text-tertiary">{props.help}</span>
        ) : null}
      </div>
      {props.children}
    </div>
  );
}

function NumberField(props: {
  label: string;
  value: string;
  min: number;
  max?: number;
  step: number;
  onChange: (value: string) => void;
}) {
  return (
    <Field label={props.label}>
      <Input
        type="number"
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
      />
    </Field>
  );
}

function ShellCommandEnvHelp() {
  const envs = [
    ['SLASHA_ALERT_DETAIL', 'System-generated alert description'],
    ['SLASHA_ALERT_VALUE', 'Current value'],
    ['SLASHA_ALERT_KIND', 'Alert kind (server_cpu, app_memory, etc.)'],
    ['SLASHA_ALERT_RULE_NAME', 'Rule name'],
    ['SLASHA_ALERT_STATUS', 'triggered | renotified | resolved'],
  ];

  return (
    <div className="rounded-md border border-border bg-bg/40 px-3 py-2">
      <p className="text-[11px] text-text-tertiary">
        Runs with <code className="font-mono text-text">sh -lc</code> and these
        envs:
      </p>
      <div className="mt-2 grid gap-1">
        {envs.map(([name, description]) => (
          <div
            key={name}
            className="flex items-baseline justify-between gap-3 text-[11px]"
          >
            <code className="font-mono text-text-secondary">{name}</code>
            <span className="text-text-tertiary">{description}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function TemplateVarHelp() {
  const vars = [
    ['{{detail}}', 'System-generated alert description'],
    ['{{value}}', 'Current metric value'],
    ['{{notification_status}}', 'triggered | renotified | resolved'],
    ['{{alert_kind}}', 'Alert kind (server_cpu, app_memory, etc.)'],
  ];

  return (
    <div className="rounded-md border border-border bg-bg/40 px-3 py-2">
      <p className="text-[11px] text-text-tertiary">Available variables:</p>
      <div className="mt-2 grid gap-1">
        {vars.map(([name, description]) => (
          <div
            key={name}
            className="flex items-baseline justify-between gap-3 text-[11px]"
          >
            <code className="font-mono text-text-secondary">{name}</code>
            <span className="text-text-tertiary">{description}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
