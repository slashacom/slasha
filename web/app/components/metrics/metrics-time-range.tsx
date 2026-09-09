import { HStack } from '~/components/interface/stacks';
import { type TimeRange, TIME_RANGES } from '~/utils/metrics-utils';
import { cn } from '~/utils/classname';

type MetricsTimeRangeProps = {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  isLoading?: boolean;
};

export function MetricsTimeRange(props: MetricsTimeRangeProps) {
  const { value, onChange, isLoading } = props;

  return (
    <HStack space={1} className="rounded border border-border bg-surface p-0.5">
      {TIME_RANGES.map((range) => (
        <button
          key={range.hours}
          type="button"
          onClick={() => onChange(range)}
          className={cn(
            'h-7 rounded px-3 text-[11px] font-medium transition-colors',
            value.hours === range.hours
              ? 'bg-white/[0.08] text-text'
              : 'text-text-tertiary hover:text-text',
            isLoading && 'opacity-70'
          )}
        >
          {range.label}
        </button>
      ))}
    </HStack>
  );
}
