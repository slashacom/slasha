import { Bell, ShieldAlert } from 'lucide-react';
import { EmptyPage } from '~/components/global/empty-page';
import { cn } from '~/utils/classname';

type AlertEmptyStateProps = {
  type: 'incidents' | 'notifications';
  className?: string;
};

export function AlertEmptyState(props: AlertEmptyStateProps) {
  const { type, className } = props;

  if (type === 'incidents') {
    return (
      <EmptyPage
        icon={ShieldAlert}
        title="No incidents yet."
        subtitle="Open incidents will appear here when a rule first triggers."
        className={cn('min-h-[320px]', className)}
      />
    );
  }

  return (
    <EmptyPage
      icon={Bell}
      title="No notifications yet."
      subtitle="When a rule fires, each delivery attempt will appear here."
      className={cn('min-h-[320px]', className)}
    />
  );
}
