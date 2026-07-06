import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { FormField } from '~/components/interface/form-field';
import { Input } from '~/components/interface/input';
import { Select } from '~/components/interface/select';
import { Switch } from '~/components/interface/switch';
import type { AlertChannel, AlertChannelConfig } from '~/models/alerts';
import { useCreateAlertChannel, useUpdateAlertChannel } from '~/queries/alerts';
import {
  alertChannelKinds,
  alertChannelRegistry,
  buildChannelConfig,
  channelDraftFromChannel,
  emptyChannelDraft,
} from './alert-definitions';

type AlertChannelFormProps = {
  channel?: AlertChannel;
  onCancel: () => void;
  onSaved: () => void;
};

export function AlertChannelForm(props: AlertChannelFormProps) {
  const { channel, onCancel, onSaved } = props;
  const createChannel = useCreateAlertChannel();
  const updateChannel = useUpdateAlertChannel();
  const [draft, setDraft] = useState(() =>
    channel ? channelDraftFromChannel(channel) : emptyChannelDraft()
  );

  const handleSave = async () => {
    const name = draft.name.trim();
    if (!name) {
      toast.error('Channel name is required.');
      return;
    }

    const result = buildChannelConfig(draft);
    if ('error' in result) {
      toast.error(result.error);
      return;
    }

    const payload = {
      name,
      config: result.config,
      enabled: draft.enabled,
    };
    const promise = channel
      ? updateChannel.mutateAsync({ id: channel.id, data: payload })
      : createChannel.mutateAsync(payload);

    toast.promise(promise, {
      loading: channel ? 'Updating channel...' : 'Creating channel...',
      success: channel ? 'Channel updated.' : 'Channel created.',
      error: (error) => error.message || 'Failed to save channel.',
    });

    try {
      await promise;
      onSaved();
    } catch {
      return;
    }
  };

  return (
    <div className="max-w-xl space-y-5">
      <FormField label="Name">
        <Input
          value={draft.name}
          onChange={(event) =>
            setDraft((current) => ({ ...current, name: event.target.value }))
          }
          placeholder="Production Slack"
        />
      </FormField>

      <FormField label="Kind">
        <Select
          value={draft.kind}
          onChange={(event) => {
            const kind = event.target.value as AlertChannelConfig['kind'];
            setDraft((current) => ({
              ...emptyChannelDraft(kind),
              id: current.id,
              name: current.name,
              enabled: current.enabled,
            }));
          }}
        >
          {alertChannelKinds.map((kind) => (
            <option key={kind} value={kind}>
              {alertChannelRegistry[kind].label}
            </option>
          ))}
        </Select>
        <p className="text-xs text-text-tertiary">
          {alertChannelRegistry[draft.kind].description}
        </p>
      </FormField>

      {draft.kind === 'slack' || draft.kind === 'discord' ? (
        <FormField
          label={`${draft.kind === 'slack' ? 'Slack' : 'Discord'} webhook URL`}
        >
          <Input
            value={draft.webhook_url}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                webhook_url: event.target.value,
              }))
            }
            placeholder={
              draft.kind === 'slack'
                ? 'https://hooks.slack.com/services/...'
                : 'https://discord.com/api/webhooks/...'
            }
          />
        </FormField>
      ) : draft.kind === 'telegram' ? (
        <>
          <FormField label="Bot token">
            <Input
              value={draft.bot_token}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  bot_token: event.target.value,
                }))
              }
              placeholder="123456:ABCDEF..."
            />
          </FormField>
          <FormField label="Chat id">
            <Input
              value={draft.chat_id}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  chat_id: event.target.value,
                }))
              }
              placeholder="-1001234567890"
            />
          </FormField>
        </>
      ) : (
        <>
          <FormField label="SMTP Host">
            <Input
              value={draft.smtp_host}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  smtp_host: event.target.value,
                }))
              }
              placeholder="smtp.example.com"
            />
          </FormField>
          <FormField label="SMTP Port">
            <Input
              value={draft.smtp_port}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  smtp_port: event.target.value,
                }))
              }
              placeholder="587"
            />
          </FormField>
          <FormField label="SMTP Username">
            <Input
              value={draft.smtp_username}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  smtp_username: event.target.value,
                }))
              }
              placeholder=""
            />
          </FormField>
          <FormField label="SMTP Password">
            <Input
              value={draft.smtp_password}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  smtp_password: event.target.value,
                }))
              }
              placeholder=""
              type="password"
            />
          </FormField>
          <FormField label="From address">
            <Input
              value={draft.from_address}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  from_address: event.target.value,
                }))
              }
              placeholder="alerts@your-domain.com"
            />
          </FormField>
          <FormField label="To address">
            <Input
              value={draft.to_address}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  to_address: event.target.value,
                }))
              }
              placeholder="you@example.com"
            />
          </FormField>
        </>
      )}

      <div className="flex items-center justify-between border-t border-border pt-4">
        <div>
          <p className="text-sm font-medium text-text">Enabled</p>
          <p className="text-xs text-text-tertiary">
            Disabled channels remain saved but receive no alerts.
          </p>
        </div>
        <Switch
          checked={draft.enabled}
          onCheckedChange={(enabled) =>
            setDraft((current) => ({ ...current, enabled }))
          }
        />
      </div>

      <div className="flex items-center gap-2 pt-2">
        <Button
          label={channel ? 'Save changes' : 'Create channel'}
          onClick={handleSave}
          isLoading={createChannel.isPending || updateChannel.isPending}
        />
        <Button label="Cancel" variant="ghost" onClick={onCancel} />
      </div>
    </div>
  );
}
