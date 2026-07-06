import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { FormField } from '~/components/interface/form-field';
import { FormSection } from '~/components/interface/form-section';
import { Input } from '~/components/interface/input';
import { NumberField } from '~/components/interface/number-field';
import { Select } from '~/components/interface/select';
import { Switch } from '~/components/interface/switch';
import { Textarea } from '~/components/interface/textarea';
import type { AlertChannel, AlertRule, AlertRuleConfig } from '~/models/alerts';
import type { App } from '~/models/app';
import type { CronJob } from '~/models/cron';
import { useCreateAlertRule, useUpdateAlertRule } from '~/queries/alerts';
import {
  alertRuleKinds,
  alertRuleRegistry,
  buildRuleConfig,
  DEFAULT_ALERT_COOLDOWN_SECS,
  emptyRuleDraft,
  ruleDraftFromRule,
} from './alert-definitions';
import { AlertChannelMultiSelect } from './alert-channel-multi-select';
import { ShellCommandEnvHelp } from './shell-command-env-help';
import { TemplateVarHelp } from './template-var-help';

type AlertRuleFormProps = {
  apps: App[];
  channels: AlertChannel[];
  crons: CronJob[];
  rule?: AlertRule;
  onCancel: () => void;
  onSaved: () => void;
};

export function AlertRuleForm(props: AlertRuleFormProps) {
  const { apps, channels, crons, rule, onCancel, onSaved } = props;
  const createRule = useCreateAlertRule();
  const updateRule = useUpdateAlertRule();
  const [draft, setDraft] = useState(() =>
    rule
      ? ruleDraftFromRule(rule)
      : emptyRuleDraft('server_cpu', DEFAULT_ALERT_COOLDOWN_SECS)
  );

  const handleSave = async () => {
    const name = draft.name;
    if (!name) {
      toast.error('Rule name is required.');
      return;
    }

    const cooldownSecs = Number(draft.cooldown_secs);
    if (!cooldownSecs || cooldownSecs <= 0) {
      toast.error('Cooldown must be greater than zero.');
      return;
    }

    const config = buildRuleConfig(draft);

    const payload = {
      name,
      config,
      channel_ids: draft.channel_ids,
      direct_webhook_url: draft.direct_webhook_url || undefined,
      message_template: draft.message_template || undefined,
      shell_command: draft.shell_command || undefined,
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
        <FormField label="Name">
          <Input
            value={draft.name}
            onChange={(event) =>
              setDraft((current) => ({ ...current, name: event.target.value }))
            }
            placeholder="Server CPU warning"
          />
        </FormField>

        <FormField
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
        </FormField>

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
            <FormField label="App">
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
            </FormField>
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
            <FormField label="Domain">
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
            </FormField>
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
          <FormField label="Domain">
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
          </FormField>
        ) : null}

        {draft.kind === 'app_health_check' ? (
          <>
            <FormField label="App">
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
            </FormField>
            <FormField label="Health check URL">
              <Input
                value={draft.health_check_url}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    health_check_url: event.target.value,
                  }))
                }
                placeholder="https://myapp.example.com/health"
              />
            </FormField>
          </>
        ) : null}

        {draft.kind === 'cron_failed' ? (
          <FormField label="Cron job">
            <Select
              value={draft.cron_job_id}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  cron_job_id: event.target.value,
                }))
              }
            >
              <option value="">Select a cron job</option>
              {crons.map((cron) => {
                const app = apps.find((item) => item.id === cron.app_id);
                return (
                  <option key={cron.id} value={cron.id}>
                    {app ? `${app.name}: ${cron.name}` : cron.name}
                  </option>
                );
              })}
            </Select>
          </FormField>
        ) : null}
      </FormSection>

      <FormSection
        title="Delivery"
        description="Select reusable channels or configure an optional direct action."
      >
        <FormField label="Channels">
          <AlertChannelMultiSelect
            channels={channels}
            selectedIds={draft.channel_ids}
            onChange={(channel_ids) =>
              setDraft((current) => ({ ...current, channel_ids }))
            }
          />
        </FormField>

        <FormField label="Direct webhook URL" help="Optional">
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
        </FormField>

        <FormField label="Shell command" help="Optional">
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
        </FormField>
      </FormSection>

      <FormSection
        title="Behavior"
        description="Control notification content and repeat frequency."
      >
        <FormField label="Message template" help="Optional">
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
        </FormField>

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
