import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Bell, Plus, Pencil, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import {
  configSummary,
  deliverySummary,
} from '~/components/alerts/alert-definitions';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { SectionHeader } from '~/components/interface/section-header';
import { Table } from '~/components/interface/table';
import type { AlertRule } from '~/models/alerts';
import { getAppsOptions } from '~/queries/apps';
import {
  getAlertChannelsOptions,
  getAlertRulesOptions,
  useDeleteAlertRule,
} from '~/queries/alerts';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getAlertChannelsOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function AlertsRulesPage() {
  const navigate = useNavigate();
  const { data: rulesData } = useSuspenseQuery(getAlertRulesOptions());
  const { data: channelsData } = useSuspenseQuery(getAlertChannelsOptions());
  const { data: appsData } = useSuspenseQuery(getAppsOptions());
  const deleteRule = useDeleteAlertRule();
  const [ruleToDelete, setRuleToDelete] = useState<AlertRule | null>(null);
  const apps = appsData.apps.map((item) => item.app);
  const channelsById = useMemo(
    () =>
      new Map(channelsData.channels.map((channel) => [channel.id, channel])),
    [channelsData.channels]
  );

  return (
    <div className="p-8">
      <SectionHeader
        icon={Bell}
        title="Rules"
        description="Manage alert conditions and their delivery behavior."
        actions={
          <Button
            to="/alerts/rules/new"
            label="New rule"
            icon={<Plus className="size-4" />}
          />
        }
        className="h-auto border-0 px-0"
      />

      <div className="mt-8">
        {rulesData.rules.length === 0 ? (
          <EmptyPage
            icon={Bell}
            title="No rules yet."
            subtitle="Create a rule to start monitoring your server and apps."
            actionLabel="Create rule"
            actionIcon={<Plus className="size-4" />}
            onAction={() => navigate('/alerts/rules/new')}
            className="min-h-[320px]"
          />
        ) : (
          <div className="rounded-lg border border-border bg-surface p-6">
            <div className="overflow-x-auto">
              <Table
                columns={[
                  'Name',
                  'Kind',
                  'Delivery',
                  'Cooldown',
                  'Status',
                  { label: '', align: 'right' },
                ]}
              >
                {rulesData.rules.map((rule) => (
                  <tr key={rule.id}>
                    <td className="py-3 pr-4">
                      <div className="font-medium text-text">{rule.name}</div>
                      <div className="mt-1 text-xs text-text-tertiary">
                        {configSummary(rule, apps)}
                      </div>
                    </td>
                    <td className="py-3 pr-4 capitalize text-text-secondary">
                      {rule.config.kind.replaceAll('_', ' ')}
                    </td>
                    <td className="py-3 pr-4 text-text-secondary">
                      {deliverySummary(rule, channelsById)}
                    </td>
                    <td className="py-3 pr-4 text-text-secondary">
                      {rule.cooldown_secs}s
                    </td>
                    <td className="py-3 pr-4">
                      <AlertStatusBadge state={rule.enabled ? 'ok' : 'muted'}>
                        {rule.enabled ? 'Enabled' : 'Disabled'}
                      </AlertStatusBadge>
                    </td>
                    <td className="py-3 text-right">
                      <div className="flex items-center justify-end gap-3">
                        <Link
                          to={`/alerts/rules/${rule.id}/edit`}
                          className="text-text-secondary transition-colors hover:text-text"
                          title="Edit rule"
                        >
                          <Pencil className="size-4" />
                        </Link>
                        <button
                          type="button"
                          title="Delete rule"
                          onClick={() => setRuleToDelete(rule)}
                          className="text-red-400/80 transition-colors hover:text-red-400"
                        >
                          <Trash2 className="size-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </Table>
            </div>
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={ruleToDelete !== null}
        onOpenChange={(open) => !open && setRuleToDelete(null)}
        title="Delete rule"
        description={
          ruleToDelete
            ? `Delete ${ruleToDelete.name}? This will stop future evaluations.`
            : ''
        }
        confirmLabel="Delete"
        onConfirm={async () => {
          if (!ruleToDelete) {
            return;
          }

          try {
            const promise = deleteRule.mutateAsync(ruleToDelete.id);
            toast.promise(promise, {
              loading: 'Deleting rule...',
              success: 'Rule deleted.',
              error: (error) => error.message || 'Failed to delete rule.',
            });
            await promise;
            setRuleToDelete(null);
          } catch {
            return;
          }
        }}
      />
    </div>
  );
}
