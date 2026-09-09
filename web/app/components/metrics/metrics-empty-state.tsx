import { Activity } from 'lucide-react';
import { EmptyPage } from '~/components/global/empty-page';

type MetricsEmptyStateProps = {
  description: string;
};

export function MetricsEmptyState(props: MetricsEmptyStateProps) {
  const { description } = props;

  return (
    <EmptyPage
      icon={Activity}
      size="lg"
      title="Waiting for the first data points."
      subtitle={description}
    />
  );
}
