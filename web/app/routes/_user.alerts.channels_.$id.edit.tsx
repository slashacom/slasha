import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertChannelForm } from '~/components/alerts/alert-channel-form';
import { getAlertChannelsOptions } from '~/queries/alerts';
import { queryClient } from '~/utils/query-client';
import { PageHeader } from '~/components/interface/page-header';

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
      <div className="px-8 py-6 text-sm text-text-secondary">
        Alert channel not found.
      </div>
    );
  }

  return (
    <div className="px-8 py-6">
      <PageHeader
        className="mb-8"
        title="Edit channel"
        description={`Update ${channel.name} and its delivery configuration.`}
      />
      <AlertChannelForm
        channel={channel}
        onCancel={() => navigate('/alerts/channels')}
        onSaved={() => navigate('/alerts/channels')}
      />
    </div>
  );
}
