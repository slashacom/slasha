import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ShieldAlert } from 'lucide-react';
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
import { formatDateTime, formatDuration } from '~/utils/date';
import { incidentValues } from '~/components/alerts/incident-values';
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
  const values = incidentValues(incident);
  const firstNotification = notifications[0];
  const latestNotification = notifications[notifications.length - 1];

  return (
    <div className="space-y-8 px-8 py-6">
      <SectionHeader
        backTo="/alerts"
        icon={ShieldAlert}
        title={ruleName}
        description={ruleCondition ?? undefined}
        actions={
          <Button label="Refresh" variant="ghost" onClick={() => refetch()} />
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
          value={formatDateTime(incident.opened_at)}
          mono={false}
        />
        <AlertStat
          label={incident.status === 'open' ? 'Ongoing for' : 'Duration'}
          value={formatDuration(incident.opened_at, incident.resolved_at)}
          mono={false}
        />
        <AlertStat
          label="Resolved"
          value={formatDateTime(incident.resolved_at)}
          mono={false}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <AlertCard
          title="Incident details"
          description="Threshold and routing metadata for this alert entry."
        >
          <div className="grid gap-3 sm:grid-cols-2">
            <AlertDetailStat label="Rule" value={ruleName} />
            {ruleCondition ? (
              <AlertDetailStat label="Condition" value={ruleCondition} />
            ) : null}
            {values.map((entry) => (
              <AlertDetailStat
                key={entry.label}
                label={entry.label}
                value={entry.value}
              />
            ))}
          </div>
        </AlertCard>

        <AlertCard
          title="Trigger summary"
          description="Every recorded trigger, re-notify, and resolution event for this incident."
        >
          <div className="grid gap-3 sm:grid-cols-2">
            <AlertDetailStat
              label="Trigger count"
              value={String(notifications.length)}
            />
            <AlertDetailStat
              label="Latest event"
              value={
                latestNotification
                  ? formatNotificationKind(latestNotification.kind)
                  : '—'
              }
            />
            <AlertDetailStat
              label="First trigger"
              value={formatDateTime(firstNotification?.created_at)}
            />
            <AlertDetailStat
              label="Last trigger"
              value={formatDateTime(latestNotification?.created_at)}
            />
          </div>
        </AlertCard>
      </div>

      <AlertCard
        title="Triggers"
        description="Click a trigger entry to inspect the full message and payload."
      >
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
                    {formatDateTime(notification.created_at)}
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
