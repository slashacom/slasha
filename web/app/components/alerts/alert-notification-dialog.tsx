import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import type { AlertNotification } from '~/models/alerts';
import { formatDateTime } from '~/utils/date';
import { AlertDetailStat } from './alert-detail-stat';
import { AlertMessage } from './alert-message';
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
            <div className="rounded-md border border-border bg-bg/40 p-4">
              <AlertMessage message={notification.message} />
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <AlertDetailStat
                label="Kind"
                value={formatNotificationKind(notification.kind)}
              />
              <AlertDetailStat
                label="Delivered"
                value={formatDateTime(notification.created_at)}
              />
            </div>

            <AlertDetailStat
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
