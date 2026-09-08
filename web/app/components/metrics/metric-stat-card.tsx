import { HStack } from '~/components/interface/stacks';

type MetricStatCardProps = {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  value: string;
  hint?: React.ReactNode;
};

export function MetricStatCard(props: MetricStatCardProps) {
  const { label, icon: Icon, value, hint } = props;

  return (
    <div className="min-w-0 rounded-lg border border-border bg-surface p-4">
      <HStack
        justifyContent="between"
        className="text-xs font-medium text-text-tertiary"
      >
        <span>{label}</span>
        <Icon className="size-4 shrink-0" />
      </HStack>
      <div className="mt-2 truncate text-2xl font-semibold tracking-tight text-text">
        {value}
      </div>
      {hint ? (
        <div className="mt-1 text-[11px] text-text-tertiary">{hint}</div>
      ) : null}
    </div>
  );
}
