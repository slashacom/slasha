import { Button } from '~/components/interface/button';

type DangerZoneProps = {
  description: string;
  actionTitle: string;
  actionDescription: string;
  actionLabel: string;
  onAction: () => void;
};

export function DangerZone(props: DangerZoneProps) {
  const { description, actionTitle, actionDescription, actionLabel, onAction } =
    props;

  return (
    <div>
      <h3 className="text-[14px] font-semibold text-text">Danger Zone</h3>
      <p className="mt-1 text-[13px] text-text-tertiary">{description}</p>

      <div className="mt-6 rounded-lg border border-red-500/20 bg-red-500/5 p-6">
        <div className="flex items-center justify-between gap-6">
          <div>
            <h4 className="text-[13px] font-medium text-red-500">
              {actionTitle}
            </h4>
            <p className="mt-1 max-w-prose text-pretty text-[12px] text-red-500/70">
              {actionDescription}
            </p>
          </div>
          <Button
            label={actionLabel}
            color="error"
            size="sm"
            className="shrink-0"
            onClick={onAction}
          />
        </div>
      </div>
    </div>
  );
}
