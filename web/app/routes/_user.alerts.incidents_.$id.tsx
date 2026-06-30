import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, ShieldAlert } from 'lucide-react';
import { useParams } from 'react-router';
import { AlertCard } from '~/components/alerts/alert-card';
import { AlertDetailStat } from '~/components/alerts/alert-detail-stat';
import { AlertEmptyState } from '~/components/alerts/alert-empty-state';
import { AlertNotificationDialog } from '~/components/alerts/alert-notification-dialog';
import { AlertNotificationPreview } from '~/components/alerts/alert-notification-preview';
import { AlertStat } from '~/components/alerts/alert-stat';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { configSummary } from '~/components/alerts/alert-definitions';
import { formatNotificationKind } from '~/components/alerts/notification-kind';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { Table } from '~/components/interface/table';
import type { AlertNotification } from '~/models/alerts';
import { getAppsOptions } from '~/queries/apps';
import {
  getAlertIncidentNotificationsOptions,
  getAlertRulesOptions,
} from '~/queries/alerts';
import { formatDate, formatMetric } from '~/utils/format';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(
      getAlertIncidentNotificationsOptions(params.id)
    ),
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function AlertIncidentDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data, refetch } = useSuspenseQuery(
    getAlertIncidentNotificationsOptions(id!)
  );
  const { data: rulesData } = useSuspenseQuery(getAlertRulesOptions());
  const { data: appsData } = useSuspenseQuery(getAppsOptions());
  const [selectedNotification, setSelectedNotification] =
    useState<AlertNotification | null>(null);
  const { incident, notifications } = data;
  const rule = rulesData.rules.find((item) => item.id === incident.rule_id);
  const apps = appsData.apps.map((item) => item.app);
  const ruleName = rule?.name ?? 'Unknown rule';
  const ruleCondition = rule ? configSummary(rule, apps) : null;

  return (
    <div className="space-y-8 p-8">
      <SectionHeader
        icon={ShieldAlert}
        title={ruleName}
        description={ruleCondition ?? undefined}
        actions={
          <>
            <Button
              to="/alerts"
              label="Back to alerts"
              variant="ghost"
              icon={<ArrowLeft className="size-4" />}
            />
            <Button label="Refresh" variant="ghost" onClick={() => refetch()} />
          </>
        }
        className="h-auto border-0 px-0"
      />

      <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-4">
        <AlertStat
          label="Status"
          value={
            <AlertStatusBadge
              state={incident.status === 'open' ? 'warn' : 'ok'}
            >
              {incident.status}
            </AlertStatusBadge>
          }
          valueClassName="mt-2"
        />
        <AlertStat
          label="Opened"
          value={formatDate(incident.opened_at)}
          mono={false}
        />
        <AlertStat
          label="Last seen"
          value={formatDate(incident.last_notified_at)}
          mono={false}
        />
        <AlertStat
          label="Resolved"
          value={formatDate(incident.resolved_at)}
          mono={false}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <AlertCard>
          <div className="mb-4">
            <h3 className="text-xs font-medium text-text-tertiary">
              Incident details
            </h3>
            <p className="mt-1 text-[11px] text-text-tertiary">
              Threshold and routing metadata for this alert entry.
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <AlertDetailStat label="Rule" value={ruleName} />
            {ruleCondition ? (
              <AlertDetailStat label="Condition" value={ruleCondition} />
            ) : null}
            <AlertDetailStat
              label="Trigger value"
              value={formatMetric(incident.trigger_value)}
            />
            <AlertDetailStat
              label="Current value"
              value={formatMetric(incident.current_value)}
            />
            <AlertDetailStat
              label="Recovery value"
              value={formatMetric(incident.recovery_value)}
            />
            <AlertDetailStat
              label="Threshold value"
              value={formatMetric(incident.threshold_value)}
            />
          </div>
        </AlertCard>

        <AlertCard>
          <div className="mb-4">
            <h3 className="text-xs font-medium text-text-tertiary">
              Trigger summary
            </h3>
            <p className="mt-1 text-[11px] text-text-tertiary">
              Every recorded trigger, re-notify, and resolution event for this
              incident.
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <AlertDetailStat
              label="Trigger count"
              value={String(notifications.length)}
            />
            <AlertDetailStat
              label="Latest event"
              value={
                notifications[notifications.length - 1]
                  ? formatNotificationKind(
                      notifications[notifications.length - 1].kind
                    )
                  : '—'
              }
            />
            <AlertDetailStat
              label="Opened"
              value={formatDate(incident.opened_at)}
            />
            <AlertDetailStat
              label="Resolved"
              value={formatDate(incident.resolved_at)}
            />
          </div>
        </AlertCard>
      </div>

      <AlertCard>
        <div className="mb-5">
          <h3 className="text-xs font-medium text-text-tertiary">Triggers</h3>
          <p className="mt-1 text-[11px] text-text-tertiary">
            Click a trigger entry to inspect the full message and payload.
          </p>
        </div>

        {notifications.length === 0 ? (
          <AlertEmptyState type="notifications" />
        ) : (
          <div className="overflow-x-auto">
            <Table
              columns={[
                'Event',
                'Summary',
                'Created',
                { label: '', align: 'right' },
              ]}
            >
              {notifications.map((notification) => (
                <tr key={notification.id}>
                  <td className="py-4 pr-4 align-top">
                    <div className="inline-flex rounded-full border border-border bg-bg/60 px-2.5 py-1 text-[11px] font-medium text-text-secondary">
                      {formatNotificationKind(notification.kind)}
                    </div>
                  </td>
                  <td className="py-4 pr-4">
                    <AlertNotificationPreview
                      message={notification.message}
                      className="max-w-[720px]"
                    />
                  </td>
                  <td className="py-4 text-text-secondary">
                    {formatDate(notification.created_at)}
                  </td>
                  <td className="py-4 text-right">
                    <Button
                      label="Details"
                      variant="ghost"
                      size="sm"
                      onClick={() => setSelectedNotification(notification)}
                    />
                  </td>
                </tr>
              ))}
            </Table>
          </div>
        )}
      </AlertCard>

      <AlertNotificationDialog
        notification={selectedNotification}
        open={selectedNotification !== null}
        onOpenChange={(open) => !open && setSelectedNotification(null)}
      />
    </div>
  );
}
