import { cn } from '~/utils/classname';

type ParsedAlertMessage = {
  title: string | null;
  fields: Array<{ label: string; value: string }>;
  lines: string[];
};

function parseAlertMessage(message: string): ParsedAlertMessage {
  const lines = message
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  const fields: Array<{ label: string; value: string }> = [];
  const body: string[] = [];
  let title: string | null = null;

  for (const line of lines) {
    const titleMatch = line.match(/^\*([^*]+)\*$/);
    if (!title && titleMatch) {
      title = titleMatch[1];
      continue;
    }

    const fieldMatch = line.match(/^\*([^*]+):\*\s*(.+)$/);
    if (fieldMatch) {
      fields.push({
        label: fieldMatch[1],
        value: fieldMatch[2],
      });
      continue;
    }

    body.push(line.replaceAll('*', ''));
  }

  return { title, fields, lines: body };
}

export function NotificationMessagePreview(props: {
  message: string;
  className?: string;
}) {
  const parsed = parseAlertMessage(props.message);
  const previewFields = parsed.fields.slice(0, 2);

  return (
    <div className={cn('min-w-0 space-y-1', props.className)}>
      <div className="truncate text-sm font-semibold tracking-tight text-text">
        {parsed.title ?? parsed.lines[0] ?? 'Notification'}
      </div>
      {previewFields.length > 0 ? (
        <div className="truncate text-[11px] text-text-tertiary">
          {previewFields
            .map((field) => `${field.label}: ${field.value}`)
            .join(' · ')}
        </div>
      ) : parsed.lines[0] ? (
        <div className="truncate text-[11px] text-text-tertiary">
          {parsed.lines[0]}
        </div>
      ) : null}
    </div>
  );
}
