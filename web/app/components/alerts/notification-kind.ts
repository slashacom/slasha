import type { AlertNotification } from '~/models/alerts';

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
