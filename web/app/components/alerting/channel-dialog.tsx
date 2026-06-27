import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '../interface/button';
import { Input } from '../interface/input';
import { Label } from '../interface/label';
import { Select } from '../interface/select';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../interface/dialog';
import type { Channel } from '~/models/channel';
import { CHANNEL_KINDS, type ChannelKind, parseJson } from './catalog';
import { useCreateChannel, useUpdateChannel } from '~/queries/alerting';

type ChannelDialogProps = {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  channel?: Channel;
};

type ChannelConfig = {
  webhook_url?: string;
  bot_token?: string;
  chat_id?: string;
};

export function ChannelDialog(props: ChannelDialogProps) {
  const { isOpen, onOpenChange, channel } = props;
  const isEdit = Boolean(channel);

  const createChannel = useCreateChannel();
  const updateChannel = useUpdateChannel();

  const initialConfig = channel ? parseJson<ChannelConfig>(channel.config) : {};
  const [name, setName] = useState(channel?.name ?? '');
  const [kind, setKind] = useState<ChannelKind>(
    (channel?.kind as ChannelKind) ?? 'slack'
  );
  const [config, setConfig] = useState<ChannelConfig>(initialConfig);

  const setConfigField = (key: keyof ChannelConfig, value: string) => {
    setConfig((prev) => {
      return { ...prev, [key]: value };
    });
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const cleanConfig =
      kind === 'slack'
        ? { webhook_url: config.webhook_url ?? '' }
        : { bot_token: config.bot_token ?? '', chat_id: config.chat_id ?? '' };

    const promise = isEdit
      ? updateChannel.mutateAsync({
          id: channel!.id,
          input: { name, config: cleanConfig },
        })
      : createChannel.mutateAsync({ name, kind, config: cleanConfig });

    toast.promise(promise, {
      loading: 'Saving channel...',
      success: `Channel ${isEdit ? 'updated' : 'created'}`,
      error: (err) => err.message || 'Failed to save channel.',
    });

    try {
      await promise;
      onOpenChange(false);
    } catch {}
  };

  const isPending = createChannel.isPending || updateChannel.isPending;

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Edit channel' : 'Add channel'}</DialogTitle>
          <DialogDescription>
            Channels are reusable delivery destinations that alert rules send
            to.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="channel-name">Name</Label>
              <Input
                id="channel-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                placeholder="e.g. Ops Slack"
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="channel-kind">Type</Label>
              <Select
                id="channel-kind"
                value={kind}
                disabled={isEdit}
                onChange={(e) => setKind(e.target.value as ChannelKind)}
              >
                {CHANNEL_KINDS.map((entry) => {
                  return (
                    <option key={entry.value} value={entry.value}>
                      {entry.label}
                    </option>
                  );
                })}
              </Select>
            </div>

            {kind === 'slack' ? (
              <div className="space-y-2">
                <Label htmlFor="webhook_url">Webhook URL</Label>
                <Input
                  id="webhook_url"
                  type="url"
                  value={config.webhook_url ?? ''}
                  onChange={(e) =>
                    setConfigField('webhook_url', e.target.value)
                  }
                  required
                  placeholder="https://hooks.slack.com/services/..."
                />
              </div>
            ) : (
              <>
                <div className="space-y-2">
                  <Label htmlFor="bot_token">Bot token</Label>
                  <Input
                    id="bot_token"
                    value={config.bot_token ?? ''}
                    onChange={(e) =>
                      setConfigField('bot_token', e.target.value)
                    }
                    required
                    placeholder="123456:ABC-DEF..."
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="chat_id">Chat ID</Label>
                  <Input
                    id="chat_id"
                    value={config.chat_id ?? ''}
                    onChange={(e) => setConfigField('chat_id', e.target.value)}
                    required
                    placeholder="-1001234567890"
                  />
                </div>
              </>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              label="Cancel"
              onClick={() => onOpenChange(false)}
            />
            <Button
              type="submit"
              label={isEdit ? 'Save channel' : 'Add channel'}
              isLoading={isPending}
              isDisabled={isPending}
            />
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
