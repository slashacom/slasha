import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Plus, Send, Trash2, Pencil, Megaphone } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import type { Channel } from '~/models/channel';
import { channelLabel } from './catalog';
import {
  getChannelsOptions,
  useDeleteChannel,
  useTestChannel,
} from '~/queries/alerting';
import { ChannelDialog } from './channel-dialog';

export function ChannelsSection() {
  const { data: channels } = useSuspenseQuery(getChannelsOptions());
  const deleteChannel = useDeleteChannel();
  const testChannel = useTestChannel();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Channel | undefined>(undefined);
  const [pendingDelete, setPendingDelete] = useState<Channel | undefined>(
    undefined
  );

  const openCreate = () => {
    setEditing(undefined);
    setDialogOpen(true);
  };

  const openEdit = (channel: Channel) => {
    setEditing(channel);
    setDialogOpen(true);
  };

  const handleTest = (channel: Channel) => {
    const promise = testChannel.mutateAsync(channel.id);
    toast.promise(promise, {
      loading: `Sending test to ${channel.name}...`,
      success: 'Test message sent',
      error: (err) => err.message || 'Test failed.',
    });
  };

  const handleDelete = () => {
    if (!pendingDelete) {
      return;
    }
    const promise = deleteChannel.mutateAsync(pendingDelete.id);
    toast.promise(promise, {
      loading: 'Deleting channel...',
      success: 'Channel deleted',
      error: (err) => err.message || 'Failed to delete channel.',
    });
    setPendingDelete(undefined);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Channels</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Reusable destinations alert rules can deliver to.
          </p>
        </div>
        <Button
          label="Add channel"
          icon={<Plus className="size-4" />}
          onClick={openCreate}
          size="sm"
        />
      </div>

      {channels.length === 0 ? (
        <EmptyPage
          dashed
          icon={Megaphone}
          title="No channels yet"
          subtitle="Add a Slack or Telegram channel for rules to deliver to."
          actionLabel="Add channel"
          actionIcon={<Plus className="size-3.5" />}
          onAction={openCreate}
        />
      ) : (
        <div className="divide-y divide-border rounded-lg border border-border bg-surface/20">
          {channels.map((channel) => {
            return (
              <div key={channel.id} className="px-4 py-3.5">
                <HStack justifyContent="between" alignItems="center">
                  <VStack space={0.5} className="min-w-0">
                    <span className="truncate text-[13px] font-medium text-text">
                      {channel.name}
                    </span>
                    <span className="text-[11px] text-text-tertiary">
                      {channelLabel(channel)}
                    </span>
                  </VStack>
                  <HStack space={1} className="shrink-0">
                    <Button
                      variant="ghost"
                      size="sm"
                      icon={<Send className="size-3.5" />}
                      label="Test"
                      onClick={() => handleTest(channel)}
                      isDisabled={testChannel.isPending}
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      icon={<Pencil className="size-3.5" />}
                      onClick={() => openEdit(channel)}
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      color="error"
                      icon={<Trash2 className="size-3.5" />}
                      onClick={() => setPendingDelete(channel)}
                    />
                  </HStack>
                </HStack>
              </div>
            );
          })}
        </div>
      )}

      {dialogOpen ? (
        <ChannelDialog
          isOpen={dialogOpen}
          onOpenChange={setDialogOpen}
          channel={editing}
        />
      ) : null}

      <ConfirmationDialog
        open={Boolean(pendingDelete)}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(undefined);
          }
        }}
        title="Delete channel"
        description={`Delete "${pendingDelete?.name}"? Rules using it will stop delivering.`}
        confirmLabel="Delete"
        onConfirm={handleDelete}
      />
    </div>
  );
}
