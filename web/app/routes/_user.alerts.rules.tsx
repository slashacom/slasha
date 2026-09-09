import { useMemo, useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Gauge, Plus, Pencil, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import {
  configSummary,
  deliverySummary,
} from '~/components/alerts/alert-definitions';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { Table, TableRow } from '~/components/interface/table';
import type { AlertRule } from '~/models/alerts';
import { getAppsOptions } from '~/queries/apps';
import {
  getAlertChannelsOptions,
  getAlertRulesOptions,
  useDeleteAlertRule,
} from '~/queries/alerts';
import { queryClient } from '~/utils/query-client';
import { Page } from '~/components/global/page';
import { TabActions } from '~/components/interface/tab-actions';
import { TableRowActions } from '~/components/interface/table-row-actions';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getAlertChannelsOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function AlertsRulesPage() {
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
    <Page>
      <TabActions>
        <Button
          to="/alerts/rules/new"
          label="New rule"
          size="sm"
          icon={<Plus className="size-3.5" />}
        />
      </TabActions>

      <p className="max-w-prose text-pretty text-sm text-text-secondary">
        Conditions Slasha watches, and how each one notifies you.
      </p>

      <div className="mt-6">
        {rulesData.rules.length === 0 ? (
          <EmptyPage
            icon={Gauge}
            size="lg"
            title="No alert rules yet."
            subtitle="A rule watches one signal — CPU, memory, disk, or app health — and notifies a channel when it crosses your threshold."
            actionLabel="Create rule"
            actionIcon={<Plus className="size-3.5" />}
            actionTo="/alerts/rules/new"
            secondaryLabel={
              channelsData.channels.length === 0 ? 'Add a channel' : undefined
            }
            secondaryTo={
              channelsData.channels.length === 0
                ? '/alerts/channels/new'
                : undefined
            }
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
                  <TableRow key={rule.id} to={`/alerts/rules/${rule.id}/edit`}>
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
                      <TableRowActions
                        actions={[
                          {
                            label: 'Edit rule',
                            icon: Pencil,
                            to: `/alerts/rules/${rule.id}/edit`,
                          },
                          {
                            label: 'Delete rule',
                            icon: Trash2,
                            isDestructive: true,
                            onClick: () => setRuleToDelete(rule),
                          },
                        ]}
                      />
                    </td>
                  </TableRow>
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
    </Page>
  );
}
