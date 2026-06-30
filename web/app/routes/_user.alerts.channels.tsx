import { useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Plus, Webhook } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { SectionHeader } from '~/components/interface/section-header';
import { Table } from '~/components/interface/table';
import type { AlertChannel } from '~/models/alerts';
import {
  getAlertChannelsOptions,
  useDeleteAlertChannel,
} from '~/queries/alerts';
import { channelSummary } from '~/components/alerts/alert-definitions';
import { formatDate } from '~/utils/format';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAlertChannelsOptions());
  return null;
}

export default function AlertsChannelsPage() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getAlertChannelsOptions());
  const deleteChannel = useDeleteAlertChannel();
  const [channelToDelete, setChannelToDelete] = useState<AlertChannel | null>(
    null
  );

  return (
    <div className="p-8">
      <SectionHeader
        icon={Webhook}
        title="Channels"
        description="Manage reusable destinations for alert delivery."
        actions={
          <Button
            to="/alerts/channels/new"
            label="New channel"
            icon={<Plus className="size-4" />}
          />
        }
        className="h-auto border-0 px-0"
      />

      <div className="mt-8">
        {data.channels.length === 0 ? (
          <EmptyPage
            icon={Webhook}
            title="No channels yet."
            subtitle="Create a delivery channel, then attach it to an alert rule."
            actionLabel="Create channel"
            actionIcon={<Plus className="size-4" />}
            onAction={() => navigate('/alerts/channels/new')}
            className="min-h-[320px]"
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
                      {formatDate(channel.updated_at)}
                    </td>
                    <td className="py-3 text-right">
                      <div className="flex items-center justify-end gap-3">
                        <Link
                          to={`/alerts/channels/${channel.id}/edit`}
                          className="text-xs !text-text-secondary !no-underline hover:!text-text"
                        >
                          Edit
                        </Link>
                        <button
                          type="button"
                          onClick={() => setChannelToDelete(channel)}
                          className="text-xs text-red-400 transition-colors hover:text-red-300"
                        >
                          Delete
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
