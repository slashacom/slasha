import { useMemo } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ShieldAlert } from 'lucide-react';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { AlertEmptyState } from '~/components/alerts/alert-empty-state';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { Table } from '~/components/interface/table';
import { TablePagination } from '~/components/interface/table-pagination';
import { usePagination } from '~/hooks/use-pagination';
import {
  getAlertIncidentsOptions,
  getAlertRulesOptions,
} from '~/queries/alerts';
import { formatDate, formatMetric } from '~/utils/format';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertIncidentsOptions()),
    queryClient.ensureQueryData(getAlertRulesOptions()),
  ]);
  return null;
}

export default function AlertsPage() {
  const { data, refetch } = useSuspenseQuery(getAlertIncidentsOptions());
  const { data: rulesData } = useSuspenseQuery(getAlertRulesOptions());
  const pagination = usePagination(data.incidents);
  const rulesById = useMemo(
    () => new Map(rulesData.rules.map((rule) => [rule.id, rule])),
    [rulesData.rules]
  );

  return (
    <div className="p-8">
      <SectionHeader
        icon={ShieldAlert}
        title="Alerts"
        description="Each alert entry groups its incident details and trigger history."
        actions={
          <Button label="Refresh" variant="ghost" onClick={() => refetch()} />
        }
        className="h-auto border-0 px-0"
      />

      <div className="mt-8 space-y-4">
        {data.incidents.length === 0 ? (
          <AlertEmptyState type="incidents" />
        ) : (
          <div className="rounded-lg border border-border bg-surface p-6">
            <div className="overflow-x-auto">
              <Table
                columns={[
                  'Rule',
                  'Values',
                  'Status',
                  'Opened',
                  'Last seen',
                  'Resolved',
                  { label: '', align: 'right' },
                ]}
              >
                {pagination.rows.map((incident) => (
                  <tr key={incident.id}>
                    <td className="py-4 pr-4">
                      <div className="font-medium text-text">
                        {rulesById.get(incident.rule_id)?.name ??
                          'Unknown rule'}
                      </div>
                    </td>
                    <td className="py-4 pr-4 text-text-secondary">
                      <div className="space-y-1">
                        <div>
                          Trigger {formatMetric(incident.trigger_value)} ·
                          Current {formatMetric(incident.current_value)}
                        </div>
                        <div className="text-xs text-text-tertiary">
                          Threshold {formatMetric(incident.threshold_value)}
                        </div>
                      </div>
                    </td>
                    <td className="py-4 pr-4">
                      <AlertStatusBadge
                        state={incident.status === 'open' ? 'warn' : 'ok'}
                      >
                        {incident.status}
                      </AlertStatusBadge>
                    </td>
                    <td className="py-4 pr-4 text-text-secondary">
                      {formatDate(incident.opened_at)}
                    </td>
                    <td className="py-4 pr-4 text-text-secondary">
                      {formatDate(incident.last_notified_at)}
                    </td>
                    <td className="py-4 text-text-secondary">
                      {formatDate(incident.resolved_at)}
                    </td>
                    <td className="py-4 text-right">
                      <Button
                        to={`/alerts/incidents/${incident.id}`}
                        label="View"
                        variant="ghost"
                        size="sm"
                      />
                    </td>
                  </tr>
                ))}
              </Table>
            </div>

            <div className="mt-4 flex items-center justify-between gap-4">
              <p className="text-[11px] text-text-tertiary">
                Showing {pagination.rows.length} of {data.incidents.length}{' '}
                alerts.
              </p>
              <TablePagination
                onPrevPage={pagination.previousPage}
                onNextPage={pagination.nextPage}
                disablePrev={pagination.page === 0}
                disableNext={pagination.page >= pagination.pageCount - 1}
                limit={pagination.limit}
                onLimitChange={pagination.setLimit}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
