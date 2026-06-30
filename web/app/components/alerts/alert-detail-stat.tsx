type AlertDetailStatProps = {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
};

export function AlertDetailStat(props: AlertDetailStatProps) {
  const { label, value, mono } = props;

  return (
    <div className="rounded-md border border-border bg-bg/40 p-3">
      <p className="text-xs font-medium text-text-tertiary">{label}</p>
      <div
        className={
          mono
            ? 'mt-1 break-all font-mono text-xs font-medium tracking-normal text-text-secondary'
            : 'mt-1 text-sm font-semibold tracking-tight text-text'
        }
      >
        {value}
      </div>
    </div>
  );
}
