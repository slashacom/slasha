import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import type { AlertNotification } from '~/models/alerts';
import { cn } from '~/utils/classname';
import { formatDate } from '~/utils/format';

function DetailField(props: {
  label: string;
  value: React.ReactNode;
  valueClassName?: string;
}) {
  return (
    <div className="space-y-1 rounded-md border border-border bg-bg/40 p-3">
      <p className="text-xs font-medium text-text-tertiary">{props.label}</p>
      <div
        className={cn(
          'text-base font-semibold tracking-tight text-text',
          props.valueClassName
        )}
      >
        {props.value}
      </div>
    </div>
  );
}

export function formatNotificationKind(kind: AlertNotification['kind']) {
  switch (kind) {
    case 'triggered':
      return 'Triggered';
    case 'renotified':
      return 'Re-notified';
    case 'resolved':
      return 'Resolved';
  }
}

export function NotificationDetailDialog(props: {
  notification: AlertNotification | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { notification } = props;

  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>
            {notification
              ? formatNotificationKind(notification.kind)
              : 'Notification details'}
          </DialogTitle>
          <DialogDescription>
            Review the delivery record and raw payload.
          </DialogDescription>
        </DialogHeader>

        {notification ? (
          <div className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <DetailField
                label="Kind"
                value={formatNotificationKind(notification.kind)}
              />
              <DetailField
                label="Created"
                value={formatDate(notification.created_at)}
              />
            </div>

            <DetailField
              label="Raw payload"
              value={notification.message}
              valueClassName="whitespace-pre-wrap break-words font-mono text-xs font-medium tracking-normal text-text-secondary"
            />
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
