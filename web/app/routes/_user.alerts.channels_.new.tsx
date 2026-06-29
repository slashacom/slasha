import { useNavigate } from 'react-router';
import { AlertChannelForm } from '~/components/alerts/alert-channel-form';

export default function NewAlertChannelPage() {
  const navigate = useNavigate();

  return (
    <div className="p-8">
      <div className="mb-8">
        <h2 className="text-base font-semibold text-text">New channel</h2>
        <p className="mt-1 text-sm text-text-tertiary">
          Add a reusable destination for alert notifications.
        </p>
      </div>
      <AlertChannelForm
        onCancel={() => navigate('/alerts/channels')}
        onSaved={() => navigate('/alerts/channels')}
      />
    </div>
  );
}
