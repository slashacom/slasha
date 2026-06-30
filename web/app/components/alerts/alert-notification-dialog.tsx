import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import type { AlertNotification } from '~/models/alerts';
import { formatDate } from '~/utils/format';
import { AlertDetailField } from './alert-detail-field';
import { formatNotificationKind } from './notification-kind';

type AlertNotificationDialogProps = {
  notification: AlertNotification | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function AlertNotificationDialog(props: AlertNotificationDialogProps) {
  const { notification, open, onOpenChange } = props;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
              <AlertDetailField
                label="Kind"
                value={formatNotificationKind(notification.kind)}
              />
              <AlertDetailField
                label="Created"
                value={formatDate(notification.created_at)}
              />
            </div>

            <AlertDetailField
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
