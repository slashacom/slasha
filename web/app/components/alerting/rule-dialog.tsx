import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '../interface/button';
import { Input } from '../interface/input';
import { Label } from '../interface/label';
import { Select } from '../interface/select';
import { Textarea } from '../interface/textarea';
import { Switch } from '../interface/switch';
import { HStack, VStack } from '../interface/stacks';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../interface/dialog';
import type { AlertRule } from '~/models/alert_rule';
import type { Channel } from '~/models/channel';
import { getAppsOptions } from '~/queries/apps';
import {
  ACTION_TYPES,
  ALERT_EVENTS,
  type ActionType,
  eventDef,
  parseJson,
  ruleThreshold,
} from './catalog';
import {
  useCreateRule,
  useUpdateRule,
  type RuleInput,
} from '~/queries/alerting';

type RuleDialogProps = {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  channels: Channel[];
  rule?: AlertRule;
};

type ActionConfig = {
  channel_id?: string;
  url?: string;
  command?: string;
  timeout_secs?: number;
  message?: string;
};

function targetParts(target: string): {
  mode: 'any' | 'specific';
  value: string;
} {
  const [, rest] = target.split(':');
  if (rest === undefined) {
    return { mode: 'any', value: '' };
  }
  return { mode: 'specific', value: rest };
}

export function RuleDialog(props: RuleDialogProps) {
  const { isOpen, onOpenChange, channels, rule } = props;
  const isEdit = Boolean(rule);

  const apps = useQuery(getAppsOptions());
  const createRule = useCreateRule();
  const updateRule = useUpdateRule();

  const initialConfig = rule ? parseJson<ActionConfig>(rule.action_config) : {};
  const initialTarget = rule
    ? targetParts(rule.target)
    : { mode: 'any' as const, value: '' };

  const [name, setName] = useState(rule?.name ?? '');
  const [event, setEvent] = useState(rule?.event ?? ALERT_EVENTS[0].value);
  const [threshold, setThreshold] = useState<string>(
    rule ? String(ruleThreshold(rule) ?? '') : ''
  );
  const [targetMode, setTargetMode] = useState<'any' | 'specific'>(
    initialTarget.mode
  );
  const [targetValue, setTargetValue] = useState(initialTarget.value);
  const [cooldownMins, setCooldownMins] = useState<string>(
    String(Math.round((rule?.cooldown_secs ?? 900) / 60))
  );
  const [actionType, setActionType] = useState<ActionType>(
    (rule?.action_type as ActionType) ?? 'channel'
  );
  const [config, setConfig] = useState<ActionConfig>(initialConfig);
  const [enabled, setEnabled] = useState(rule?.enabled ?? true);

  const def = eventDef(event);
  const scope = def?.scope ?? 'server';

  const setConfigField = (key: keyof ActionConfig, value: string | number) => {
    setConfig((prev) => {
      return { ...prev, [key]: value };
    });
  };

  const buildTarget = (): string => {
    if (scope === 'server') {
      return 'server';
    }
    if (targetMode === 'any') {
      return scope;
    }
    return `${scope}:${targetValue}`;
  };

  const buildParams = (): Record<string, number> => {
    if (!def) {
      return {};
    }
    if (def.boolean) {
      return { gt: 0.5 };
    }
    const value = parseFloat(threshold);
    return {
      [def.comparator]: Number.isFinite(value) ? value : def.defaultThreshold,
    };
  };

  const buildActionConfig = (): Record<string, unknown> => {
    const message = config.message?.trim();
    const base: Record<string, unknown> = message ? { message } : {};

    if (actionType === 'channel') {
      return { ...base, channel_id: config.channel_id ?? '' };
    }
    if (actionType === 'webhook') {
      return { ...base, url: config.url ?? '' };
    }
    return {
      ...base,
      command: config.command ?? '',
      timeout_secs: config.timeout_secs ?? 30,
    };
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const input: RuleInput = {
      name,
      enabled,
      target: buildTarget(),
      event,
      params: buildParams(),
      cooldown_secs: Math.max(60, parseInt(cooldownMins, 10) || 15) * 60,
      action_type: actionType,
      action_config: buildActionConfig(),
    };

    const promise = isEdit
      ? updateRule.mutateAsync({ id: rule!.id, input })
      : createRule.mutateAsync(input);

    toast.promise(promise, {
      loading: 'Saving rule...',
      success: `Rule ${isEdit ? 'updated' : 'created'}`,
      error: (err) => err.message || 'Failed to save rule.',
    });

    try {
      await promise;
      onOpenChange(false);
    } catch {}
  };

  const isPending = createRule.isPending || updateRule.isPending;
  const appList = apps.data?.apps ?? [];

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Edit rule' : 'New alert rule'}</DialogTitle>
          <DialogDescription>
            When the condition is met, perform the action.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <VStack space={4} className="py-4">
            <VStack space={2}>
              <Label htmlFor="rule-name">Name</Label>
              <Input
                id="rule-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                placeholder="e.g. High CPU on web"
                autoFocus
              />
            </VStack>

            <div className="rounded-lg border border-border p-4">
              <p className="mb-3 text-[12px] font-medium uppercase tracking-wide text-text-tertiary">
                If
              </p>
              <VStack space={3}>
                <VStack space={2}>
                  <Label htmlFor="rule-event">Event</Label>
                  <Select
                    id="rule-event"
                    value={event}
                    onChange={(e) => {
                      setEvent(e.target.value);
                      setTargetMode('any');
                      setTargetValue('');
                    }}
                  >
                    {ALERT_EVENTS.map((entry) => {
                      return (
                        <option key={entry.value} value={entry.value}>
                          {entry.label}
                        </option>
                      );
                    })}
                  </Select>
                  {def ? (
                    <p className="text-[12px] text-text-tertiary">
                      {def.description}
                    </p>
                  ) : null}
                </VStack>

                {scope === 'app' ? (
                  <VStack space={2}>
                    <Label htmlFor="rule-app">App</Label>
                    <Select
                      id="rule-app"
                      value={targetMode === 'any' ? '' : targetValue}
                      onChange={(e) => {
                        const value = e.target.value;
                        setTargetMode(value ? 'specific' : 'any');
                        setTargetValue(value);
                      }}
                    >
                      <option value="">Any app</option>
                      {appList.map((item) => {
                        return (
                          <option key={item.app.id} value={item.app.id}>
                            {item.app.slug}
                          </option>
                        );
                      })}
                    </Select>
                  </VStack>
                ) : null}

                {scope === 'domain' ? (
                  <VStack space={2}>
                    <Label htmlFor="rule-domain">Domain</Label>
                    <Input
                      id="rule-domain"
                      value={targetValue}
                      onChange={(e) => {
                        const value = e.target.value;
                        setTargetValue(value);
                        setTargetMode(value ? 'specific' : 'any');
                      }}
                      placeholder="Any domain (leave blank)"
                    />
                  </VStack>
                ) : null}

                {def && !def.boolean ? (
                  <VStack space={2}>
                    <Label htmlFor="rule-threshold">
                      Threshold{def.unit ? ` (${def.unit})` : ''} — alert when{' '}
                      {def.comparator === 'gt' ? 'above' : 'below'}
                    </Label>
                    <Input
                      id="rule-threshold"
                      type="number"
                      step="0.1"
                      value={threshold}
                      onChange={(e) => setThreshold(e.target.value)}
                      required
                      placeholder={String(def.defaultThreshold)}
                    />
                  </VStack>
                ) : null}
              </VStack>
            </div>

            <div className="rounded-lg border border-border p-4">
              <p className="mb-3 text-[12px] font-medium uppercase tracking-wide text-text-tertiary">
                Then
              </p>
              <VStack space={3}>
                <VStack space={2}>
                  <Label htmlFor="rule-action">Action</Label>
                  <Select
                    id="rule-action"
                    value={actionType}
                    onChange={(e) =>
                      setActionType(e.target.value as ActionType)
                    }
                  >
                    {ACTION_TYPES.map((entry) => {
                      return (
                        <option key={entry.value} value={entry.value}>
                          {entry.label}
                        </option>
                      );
                    })}
                  </Select>
                </VStack>

                {actionType === 'channel' ? (
                  <VStack space={2}>
                    <Label htmlFor="rule-channel">Channel</Label>
                    {channels.length === 0 ? (
                      <p className="text-[12px] text-amber-500">
                        No channels configured yet. Add one above first.
                      </p>
                    ) : (
                      <Select
                        id="rule-channel"
                        value={config.channel_id ?? ''}
                        onChange={(e) =>
                          setConfigField('channel_id', e.target.value)
                        }
                        required
                      >
                        <option value="" disabled>
                          Select a channel
                        </option>
                        {channels.map((channel) => {
                          return (
                            <option key={channel.id} value={channel.id}>
                              {channel.name}
                            </option>
                          );
                        })}
                      </Select>
                    )}
                  </VStack>
                ) : null}

                {actionType === 'webhook' ? (
                  <VStack space={2}>
                    <Label htmlFor="rule-url">Webhook URL</Label>
                    <Input
                      id="rule-url"
                      type="url"
                      value={config.url ?? ''}
                      onChange={(e) => setConfigField('url', e.target.value)}
                      required
                      placeholder="https://example.com/hook"
                    />
                  </VStack>
                ) : null}

                {actionType === 'execute_program' ? (
                  <>
                    <VStack space={2}>
                      <Label htmlFor="rule-command">Command</Label>
                      <Textarea
                        id="rule-command"
                        value={config.command ?? ''}
                        onChange={(e) =>
                          setConfigField('command', e.target.value)
                        }
                        required
                        placeholder="/usr/local/bin/notify.sh"
                        className="min-h-[72px] font-mono text-xs text-text"
                      />
                      <p className="text-[12px] text-text-tertiary">
                        Runs as the slasha server user. Event details are
                        available as SLASHA_EVENT, SLASHA_TARGET, SLASHA_VALUE,
                        SLASHA_DETAIL, SLASHA_MESSAGE.
                      </p>
                    </VStack>
                    <VStack space={2}>
                      <Label htmlFor="rule-timeout">Timeout (seconds)</Label>
                      <Input
                        id="rule-timeout"
                        type="number"
                        min="1"
                        value={config.timeout_secs ?? 30}
                        onChange={(e) =>
                          setConfigField(
                            'timeout_secs',
                            parseInt(e.target.value, 10) || 30
                          )
                        }
                      />
                    </VStack>
                  </>
                ) : null}

                <VStack space={2}>
                  <Label htmlFor="rule-message">
                    Custom message (optional)
                  </Label>
                  <Textarea
                    id="rule-message"
                    value={config.message ?? ''}
                    onChange={(e) => setConfigField('message', e.target.value)}
                    placeholder="Defaults to a generated summary. Supports {{target}}, {{value}}, {{detail}}."
                    className="min-h-[60px] text-xs text-text"
                  />
                </VStack>
              </VStack>
            </div>

            <VStack space={2}>
              <Label htmlFor="rule-cooldown">Cooldown (minutes)</Label>
              <Input
                id="rule-cooldown"
                type="number"
                min="1"
                value={cooldownMins}
                onChange={(e) => setCooldownMins(e.target.value)}
              />
              <p className="text-[12px] text-text-tertiary">
                Minimum time between repeat alerts for this rule.
              </p>
            </VStack>

            <HStack justifyContent="between">
              <span className="text-sm text-text">Enabled</span>
              <Switch checked={enabled} onCheckedChange={setEnabled} />
            </HStack>
          </VStack>
          <DialogFooter>
            <Button
              variant="ghost"
              label="Cancel"
              onClick={() => onOpenChange(false)}
            />
            <Button
              type="submit"
              label={isEdit ? 'Save rule' : 'Create rule'}
              isLoading={isPending}
              isDisabled={isPending}
            />
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
