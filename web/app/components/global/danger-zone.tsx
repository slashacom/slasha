import { Button } from '~/components/interface/button';

export type DangerZoneItem = {
  title: string;
  description: string;
  label: string;
  color?: 'error' | 'neutral' | 'primary' | 'success';
  onAction: () => void;
};

type DangerZoneProps = {
  description: string;
  actionTitle?: string;
  actionDescription?: string;
  actionLabel?: string;
  onAction?: () => void;
  items?: DangerZoneItem[];
};

export function DangerZone(props: DangerZoneProps) {
  const {
    description,
    actionTitle,
    actionDescription,
    actionLabel,
    onAction,
    items,
  } = props;

  const resolvedItems: DangerZoneItem[] =
    items ??
    (actionTitle && actionLabel && onAction
      ? [
          {
            title: actionTitle,
            description: actionDescription ?? '',
            label: actionLabel,
            color: 'error',
            onAction,
          },
        ]
      : []);

  return (
    <div>
      <h3 className="text-[14px] font-semibold text-text">Danger Zone</h3>
      <p className="mt-1 text-[13px] text-text-tertiary">{description}</p>

      <div className="mt-6 divide-y divide-red-500/10 rounded-lg border border-red-500/20 bg-red-500/5">
        {resolvedItems.map((item) => (
          <div
            key={item.title}
            className="flex items-center justify-between gap-6 p-6"
          >
            <div>
              <h4 className="text-[13px] font-medium text-red-500">
                {item.title}
              </h4>
              <p className="mt-1 max-w-prose text-pretty text-[12px] text-red-500/70">
                {item.description}
              </p>
            </div>
            <Button
              label={item.label}
              color={item.color ?? 'error'}
              size="sm"
              className="shrink-0"
              onClick={item.onAction}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
