import { VStack } from '~/components/interface/stacks';
import { parseAlertMessage } from '~/utils/alert-message';
import { cn } from '~/utils/classname';

type AlertMessageProps = {
  message: string;
  className?: string;
};

export function AlertMessage(props: AlertMessageProps) {
  const { message, className } = props;
  const parsed = parseAlertMessage(message);

  return (
    <VStack space={3} className={cn('min-w-0', className)}>
      {parsed.title ? (
        <p className="text-balance text-sm font-semibold tracking-tight text-text">
          {parsed.title}
        </p>
      ) : null}

      {parsed.fields.length > 0 ? (
        <dl className="grid gap-x-6 gap-y-2 sm:grid-cols-[max-content_minmax(0,1fr)]">
          {parsed.fields.map((field) => (
            <div key={field.label} className="contents">
              <dt className="text-xs font-medium text-text-tertiary">
                {field.label}
              </dt>
              <dd className="min-w-0 break-words text-xs text-text-secondary">
                {field.value}
              </dd>
            </div>
          ))}
        </dl>
      ) : null}

      {parsed.body.map((line) => (
        <p key={line} className="text-xs text-text-secondary">
          {line}
        </p>
      ))}
    </VStack>
  );
}
