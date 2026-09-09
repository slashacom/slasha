import { useMemo } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { AlertEmptyState } from '~/components/alerts/alert-empty-state';
import { Button } from '~/components/interface/button';
import { Table, TableRow } from '~/components/interface/table';
import { TablePagination } from '~/components/interface/table-pagination';
import { usePagination } from '~/hooks/use-pagination';
import {
  getAlertIncidentsOptions,
  getAlertRulesOptions,
} from '~/queries/alerts';
import {
  formatDateTime,
  formatDuration,
  formatRelativeTime,
} from '~/utils/date';
import { incidentValueSummary } from '~/components/alerts/incident-values';
import { queryClient } from '~/utils/query-client';
import { Page } from '~/components/global/page';
import { TabActions } from '~/components/interface/tab-actions';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertIncidentsOptions()),
    queryClient.ensureQueryData(getAlertRulesOptions()),
  ]);
  return null;
}

export default function AlertsPage() {
  const { data, refetch, isFetching, dataUpdatedAt } = useSuspenseQuery(
    getAlertIncidentsOptions()
  );
  const { data: rulesData } = useSuspenseQuery(getAlertRulesOptions());
  const pagination = usePagination(data.incidents);
  const rulesById = useMemo(
    () => new Map(rulesData.rules.map((rule) => [rule.id, rule])),
    [rulesData.rules]
  );

  return (
    <Page>
      <TabActions>
        <span className="text-[11px] text-text-tertiary">
          Updated {formatRelativeTime(new Date(dataUpdatedAt))}
        </span>
        <Button
          label="Refresh"
          variant="ghost"
          size="sm"
          isLoading={isFetching}
          onClick={() => refetch()}
        />
      </TabActions>

      <p className="max-w-prose text-pretty text-sm text-text-secondary">
        Every incident groups its details and full trigger history.
      </p>

      <div className="mt-6 space-y-4">
        {data.incidents.length === 0 ? (
          <AlertEmptyState
            type="incidents"
            hasRules={rulesData.rules.length > 0}
          />
        ) : (
          <>
            <div className="-mx-8 overflow-x-auto">
              <Table
                columns={['Rule', 'Status', 'Opened', 'Duration', 'Resolved']}
              >
                {pagination.rows.map((incident) => (
                  <TableRow
                    key={incident.id}
                    to={`/alerts/incidents/${incident.id}`}
                  >
                    <td className="py-4 pr-4">
                      <div className="font-medium text-text">
                        {rulesById.get(incident.rule_id)?.name ??
                          'Unknown rule'}
                      </div>
                      {incidentValueSummary(incident) ? (
                        <div className="mt-1 text-xs text-text-tertiary">
                          {incidentValueSummary(incident)}
                        </div>
                      ) : null}
                    </td>
                    <td className="py-4 pr-4">
                      <AlertStatusBadge
                        state={incident.status === 'open' ? 'warn' : 'ok'}
                      >
                        {incident.status}
                      </AlertStatusBadge>
                    </td>
                    <td className="py-4 pr-4 text-text-secondary">
                      {formatDateTime(incident.opened_at)}
                    </td>
                    <td className="py-4 pr-4 text-text-secondary">
                      {formatDuration(incident.opened_at, incident.resolved_at)}
                    </td>
                    <td className="py-4 text-text-secondary">
                      {formatDateTime(incident.resolved_at)}
                    </td>
                  </TableRow>
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
          </>
        )}
      </div>
    </Page>
  );
}
