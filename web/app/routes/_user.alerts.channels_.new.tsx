import { useNavigate } from 'react-router';
import { AlertChannelForm } from '~/components/alerts/alert-channel-form';
import { PageHeader } from '~/components/interface/page-header';

export default function NewAlertChannelPage() {
  const navigate = useNavigate();

  return (
    <div className="px-8 py-6">
      <PageHeader
        className="mb-8"
        title="New channel"
        description="Add a reusable destination for alert notifications."
      />
      <AlertChannelForm
        onCancel={() => navigate('/alerts/channels')}
        onSaved={() => navigate('/alerts/channels')}
      />
    </div>
  );
}
