import { parseAlertMessage } from '~/utils/alert-message';
import { cn } from '~/utils/classname';

type AlertNotificationPreviewProps = {
  message: string;
  className?: string;
};

export function AlertNotificationPreview(props: AlertNotificationPreviewProps) {
  const { message, className } = props;
  const parsed = parseAlertMessage(message);
  const summary = parsed.fields
    .slice(0, 2)
    .map((field) => `${field.label}: ${field.value}`)
    .join(' · ');
  const subtitle = summary || parsed.body[0];

  return (
    <div className={cn('min-w-0 space-y-1', className)}>
      <div className="truncate text-sm font-semibold tracking-tight text-text">
        {parsed.title || 'Notification'}
      </div>
      {subtitle ? (
        <div className="truncate text-[11px] text-text-tertiary">
          {subtitle}
        </div>
      ) : null}
    </div>
  );
}
