import { useState } from 'react';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Plus, Trash2, Pencil, BellRing } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Switch } from '~/components/interface/switch';
import { HStack, VStack } from '~/components/interface/stacks';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import type { AlertRule } from '~/models/alert_rule';
import { getAppsOptions } from '~/queries/apps';
import { eventDef, parseJson, ruleThreshold } from './catalog';
import {
  getAlertRulesOptions,
  getChannelsOptions,
  ruleToInput,
  useDeleteRule,
  useUpdateRule,
} from '~/queries/alerting';
import { RuleDialog } from './rule-dialog';

export function RulesSection() {
  const { data: rules } = useSuspenseQuery(getAlertRulesOptions());
  const { data: channels } = useSuspenseQuery(getChannelsOptions());
  const apps = useQuery(getAppsOptions());

  const updateRule = useUpdateRule();
  const deleteRule = useDeleteRule();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<AlertRule | undefined>(undefined);
  const [pendingDelete, setPendingDelete] = useState<AlertRule | undefined>(
    undefined
  );

  const appSlug = (id: string): string => {
    const match = apps.data?.apps.find((item) => item.app.id === id);
    return match ? match.app.slug : id;
  };

  const describeTarget = (rule: AlertRule): string => {
    const [scope, rest] = rule.target.split(':');
    if (scope === 'server') {
      return 'server';
    }
    if (scope === 'app') {
      return rest ? `app ${appSlug(rest)}` : 'any app';
    }
    if (scope === 'domain') {
      return rest ? rest : 'any domain';
    }
    return rule.target;
  };

  const describeCondition = (rule: AlertRule): string => {
    const def = eventDef(rule.event);
    if (!def) {
      return rule.event;
    }
    if (def.boolean) {
      return `${def.label} on ${describeTarget(rule)}`;
    }
    const direction = def.comparator === 'gt' ? '>' : '<';
    return `${def.label} ${direction} ${ruleThreshold(rule)}${def.unit} on ${describeTarget(rule)}`;
  };

  const describeAction = (rule: AlertRule): string => {
    if (rule.action_type === 'channel') {
      const cfg = parseJson<{ channel_id?: string }>(rule.action_config);
      const channel = channels.find((c) => c.id === cfg.channel_id);
      return channel ? `→ ${channel.name}` : '→ (missing channel)';
    }
    if (rule.action_type === 'webhook') {
      return '→ webhook';
    }
    return '→ run command';
  };

  const openCreate = () => {
    setEditing(undefined);
    setDialogOpen(true);
  };

  const openEdit = (rule: AlertRule) => {
    setEditing(rule);
    setDialogOpen(true);
  };

  const toggleEnabled = (rule: AlertRule, enabled: boolean) => {
    const promise = updateRule.mutateAsync({
      id: rule.id,
      input: { ...ruleToInput(rule), enabled },
    });
    toast.promise(promise, {
      loading: enabled ? 'Enabling rule...' : 'Disabling rule...',
      success: enabled ? 'Rule enabled' : 'Rule disabled',
      error: (err) => err.message || 'Failed to update rule.',
    });
  };

  const handleDelete = () => {
    if (!pendingDelete) {
      return;
    }
    const promise = deleteRule.mutateAsync(pendingDelete.id);
    toast.promise(promise, {
      loading: 'Deleting rule...',
      success: 'Rule deleted',
      error: (err) => err.message || 'Failed to delete rule.',
    });
    setPendingDelete(undefined);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Alert rules</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Only enabled rules are evaluated. Each rule fires its action when
            the condition is met.
          </p>
        </div>
        <Button
          icon={<Plus className="size-4" />}
          label="New rule"
          onClick={openCreate}
          size="sm"
        />
      </div>

      {rules.length === 0 ? (
        <EmptyPage
          dashed
          icon={BellRing}
          title="No alert rules yet"
          subtitle="Create a rule to watch your server, apps, and domains."
          actionLabel="New rule"
          actionIcon={<Plus className="size-3.5" />}
          onAction={openCreate}
        />
      ) : (
        <div className="divide-y divide-border rounded-lg border border-border bg-surface/20">
          {rules.map((rule) => {
            return (
              <div key={rule.id} className="px-4 py-3.5">
                <HStack justifyContent="between" alignItems="center">
                  <HStack space={3} className="min-w-0">
                    <Switch
                      checked={rule.enabled}
                      onCheckedChange={(checked) =>
                        toggleEnabled(rule, checked)
                      }
                    />
                    <VStack space={0.5} className="min-w-0">
                      <span className="truncate text-[13px] font-medium text-text">
                        {rule.name}
                      </span>
                      <span className="text-[11px] text-text-tertiary">
                        {describeCondition(rule)} {describeAction(rule)}
                      </span>
                    </VStack>
                  </HStack>
                  <HStack space={1} className="shrink-0">
                    <Button
                      variant="ghost"
                      size="sm"
                      icon={<Pencil className="size-3.5" />}
                      onClick={() => openEdit(rule)}
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      color="error"
                      icon={<Trash2 className="size-3.5" />}
                      onClick={() => setPendingDelete(rule)}
                    />
                  </HStack>
                </HStack>
              </div>
            );
          })}
        </div>
      )}

      {dialogOpen ? (
        <RuleDialog
          isOpen={dialogOpen}
          onOpenChange={setDialogOpen}
          channels={channels}
          rule={editing}
        />
      ) : null}

      <ConfirmationDialog
        open={Boolean(pendingDelete)}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(undefined);
          }
        }}
        title="Delete rule"
        description={`Delete "${pendingDelete?.name}"?`}
        confirmLabel="Delete"
        onConfirm={handleDelete}
      />
    </div>
  );
}
