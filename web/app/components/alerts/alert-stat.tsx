import { AlertCard } from '~/components/alerts/alert-card';

type AlertStatProps = {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
  valueClassName?: string;
};

export function AlertStat(props: AlertStatProps) {
  const { label, value, mono, valueClassName } = props;

  return (
    <AlertCard className="p-5">
      <p className="text-xs font-medium text-text-tertiary">{label}</p>
      <div
        className={
          valueClassName ??
          (mono === false
            ? 'mt-2 text-lg font-semibold tracking-tight text-text'
            : 'mt-2 break-all font-mono text-sm font-semibold tracking-tight text-text')
        }
      >
        {value}
      </div>
    </AlertCard>
  );
}
