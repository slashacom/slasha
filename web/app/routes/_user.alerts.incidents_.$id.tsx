import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Bell, Clock, Gauge } from 'lucide-react';
import { useParams } from 'react-router';
import { AlertCard } from '~/components/alerts/alert-card';
import {
  IncidentCard,
  IncidentMeta,
  IncidentRow,
  IncidentTimeline,
} from '~/components/alerts/incident-meta';
import { AlertEmptyState } from '~/components/alerts/alert-empty-state';
import { AlertNotificationDialog } from '~/components/alerts/alert-notification-dialog';
import { AlertNotificationPreview } from '~/components/alerts/alert-notification-preview';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { configSummary } from '~/components/alerts/alert-definitions';
import { formatNotificationKind } from '~/components/alerts/notification-kind';
import { Button } from '~/components/interface/button';
import { PageHeader } from '~/components/interface/page-header';
import { HStack, VStack } from '~/components/interface/stacks';
import { Table } from '~/components/interface/table';
import type { AlertNotification } from '~/models/alerts';
import { getAppsOptions } from '~/queries/apps';
import {
  getAlertIncidentNotificationsOptions,
  getAlertRulesOptions,
} from '~/queries/alerts';
import { formatDateTime, formatDuration } from '~/utils/date';
import { titleCase } from '~/utils/format';
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
  const values = incidentValues(incident, rule);
  const latestNotification = notifications[notifications.length - 1];

  return (
    <div className="space-y-6 px-8 py-6">
      <PageHeader
        title={
          <HStack space={3}>
            <span>{ruleName}</span>
            <AlertStatusBadge
              state={incident.status === 'open' ? 'warn' : 'ok'}
            >
              {titleCase(incident.status)}
            </AlertStatusBadge>
          </HStack>
        }
        description={ruleCondition ?? undefined}
        actions={
          <Button
            label="Refresh"
            variant="ghost"
            size="sm"
            onClick={() => refetch()}
          />
        }
      />

      <IncidentMeta>
        <IncidentCard icon={Clock} title="Timeline">
          <IncidentTimeline
            openedAt={formatDateTime(incident.opened_at)}
            resolvedAt={
              incident.resolved_at ? formatDateTime(incident.resolved_at) : null
            }
            duration={formatDuration(incident.opened_at, incident.resolved_at)}
          />
        </IncidentCard>

        <IncidentCard icon={Gauge} title="Measurements">
          <VStack space={2}>
            {values.map((entry) => (
              <IncidentRow key={entry.label} label={entry.label}>
                {entry.value}
              </IncidentRow>
            ))}
          </VStack>
        </IncidentCard>

        <IncidentCard icon={Bell} title="Notifications">
          <VStack space={2}>
            <IncidentRow label="Sent">{notifications.length}</IncidentRow>
            <IncidentRow label="Latest">
              {latestNotification
                ? formatNotificationKind(latestNotification.kind)
                : 'None'}
            </IncidentRow>
            <IncidentRow label="Last sent">
              {formatDateTime(latestNotification?.created_at)}
            </IncidentRow>
          </VStack>
        </IncidentCard>
      </IncidentMeta>

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
