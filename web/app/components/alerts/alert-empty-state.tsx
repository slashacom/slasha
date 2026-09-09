import { Bell, Plus, ShieldCheck } from 'lucide-react';
import { EmptyPage } from '~/components/global/empty-page';

type AlertEmptyStateProps = {
  type: 'incidents' | 'notifications';
  hasRules?: boolean;
  className?: string;
};

export function AlertEmptyState(props: AlertEmptyStateProps) {
  const { type, hasRules = true, className } = props;

  if (type === 'notifications') {
    return (
      <EmptyPage
        icon={Bell}
        size="sm"
        bordered={false}
        title="No triggers recorded."
        subtitle="Every delivery attempt for this incident will be listed here."
        className={className}
      />
    );
  }

  if (!hasRules) {
    return (
      <EmptyPage
        icon={Bell}
        size="lg"
        title="Nothing is being watched yet."
        subtitle="Alert rules keep an eye on CPU, memory, disk, and app health. Create your first rule and incidents will show up here."
        actionLabel="Create rule"
        actionIcon={<Plus className="size-3.5" />}
        actionTo="/alerts/rules/new"
        secondaryLabel="Add a channel"
        secondaryTo="/alerts/channels/new"
        className={className}
      />
    );
  }

  return (
    <EmptyPage
      icon={ShieldCheck}
      size="lg"
      title="All clear — no incidents."
      subtitle="Your rules are running. The moment one trips, the incident and its trigger history land here."
      secondaryLabel="View rules"
      secondaryTo="/alerts/rules"
      className={className}
    />
  );
}
