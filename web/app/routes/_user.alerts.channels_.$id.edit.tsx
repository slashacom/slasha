import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertChannelForm } from '~/components/alerts/alert-channel-form';
import { getAlertChannelsOptions } from '~/queries/alerts';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAlertChannelsOptions());
  return null;
}

export default function EditAlertChannelPage() {
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const { data } = useSuspenseQuery(getAlertChannelsOptions());
  const channel = data.channels.find((item) => item.id === id);

  if (!channel) {
    return (
      <div className="p-8 text-sm text-text-secondary">
        Alert channel not found.
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8">
        <h2 className="text-base font-semibold text-text">Edit channel</h2>
        <p className="mt-1 text-sm text-text-tertiary">
          Update {channel.name} and its delivery configuration.
        </p>
      </div>
      <AlertChannelForm
        channel={channel}
        onCancel={() => navigate('/alerts/channels')}
        onSaved={() => navigate('/alerts/channels')}
      />
    </div>
  );
}
