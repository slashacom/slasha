import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Plus, Webhook, Send, Pencil, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';
import type { AlertChannel } from '~/models/alerts';
import {
  getAlertChannelsOptions,
  useDeleteAlertChannel,
  useTestAlertChannel,
} from '~/queries/alerts';
import { channelSummary } from '~/components/alerts/alert-definitions';
import { formatDateTime } from '~/utils/date';
import { queryClient } from '~/utils/query-client';
import { PageHeader } from '~/components/interface/page-header';
import { TableRowActions } from '~/components/interface/table-row-actions';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAlertChannelsOptions());
  return null;
}

export default function AlertsChannelsPage() {
  const { data } = useSuspenseQuery(getAlertChannelsOptions());
  const deleteChannel = useDeleteAlertChannel();
  const testChannel = useTestAlertChannel();
  const [channelToDelete, setChannelToDelete] = useState<AlertChannel | null>(
    null
  );

  return (
    <div className="p-8">
      <PageHeader
        title="Channels"
        description="Manage reusable destinations for alert delivery."
        actions={
          <Button
            to="/alerts/channels/new"
            label="New channel"
            icon={<Plus className="size-4" />}
          />
        }
      />

      <div className="mt-8">
        {data.channels.length === 0 ? (
          <EmptyPage
            icon={Webhook}
            size="lg"
            title="No delivery channels yet."
            subtitle="A channel is where alerts land — a Slack workspace, a webhook, an inbox. Create one, then attach it to a rule."
            actionLabel="Create channel"
            actionIcon={<Plus className="size-3.5" />}
            actionTo="/alerts/channels/new"
          />
        ) : (
          <div className="rounded-lg border border-border bg-surface p-6">
            <div className="overflow-x-auto">
              <Table
                columns={[
                  'Name',
                  'Kind',
                  'Status',
                  'Updated',
                  { label: '', align: 'right' },
                ]}
              >
                {data.channels.map((channel) => (
                  <tr key={channel.id}>
                    <td className="py-3 pr-4">
                      <div className="font-medium text-text">
                        {channel.name}
                      </div>
                      <div className="mt-1 text-xs text-text-tertiary">
                        {channelSummary(channel)}
                      </div>
                    </td>
                    <td className="py-3 pr-4 capitalize text-text-secondary">
                      {channel.config.kind}
                    </td>
                    <td className="py-3 pr-4">
                      <AlertStatusBadge
                        state={channel.enabled ? 'ok' : 'muted'}
                      >
                        {channel.enabled ? 'Enabled' : 'Disabled'}
                      </AlertStatusBadge>
                    </td>
                    <td className="py-3 pr-4 text-text-secondary">
                      {formatDateTime(channel.updated_at)}
                    </td>
                    <td className="py-3 text-right">
                      <TableRowActions
                        actions={[
                          {
                            label: 'Test channel',
                            icon: Send,
                            isDisabled: testChannel.isPending,
                            onClick: () => {
                              toast.promise(
                                testChannel.mutateAsync(channel.id),
                                {
                                  loading: 'Sending test message...',
                                  success: 'Test message sent.',
                                  error: (error) =>
                                    error.message ||
                                    'Failed to send test message.',
                                }
                              );
                            },
                          },
                          {
                            label: 'Edit channel',
                            icon: Pencil,
                            to: `/alerts/channels/${channel.id}/edit`,
                          },
                          {
                            label: 'Delete channel',
                            icon: Trash2,
                            isDestructive: true,
                            onClick: () => setChannelToDelete(channel),
                          },
                        ]}
                      />
                    </td>
                  </tr>
                ))}
              </Table>
            </div>
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={channelToDelete !== null}
        onOpenChange={(open) => !open && setChannelToDelete(null)}
        title="Delete channel"
        description={
          channelToDelete
            ? `Delete ${channelToDelete.name}? Existing rules will stop using it.`
            : ''
        }
        confirmLabel="Delete"
        onConfirm={async () => {
          if (!channelToDelete) {
            return;
          }

          try {
            const promise = deleteChannel.mutateAsync(channelToDelete.id);
            toast.promise(promise, {
              loading: 'Deleting channel...',
              success: 'Channel deleted.',
              error: (error) => error.message || 'Failed to delete channel.',
            });
            await promise;
            setChannelToDelete(null);
          } catch {
            return;
          }
        }}
      />
    </div>
  );
}
