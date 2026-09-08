import { Activity } from 'lucide-react';
import { VStack } from '~/components/interface/stacks';

type MetricsEmptyStateProps = {
  description: string;
};

export function MetricsEmptyState(props: MetricsEmptyStateProps) {
  const { description } = props;

  return (
    <VStack className="items-center justify-center py-20" space={4}>
      <div className="rounded-full border border-border bg-surface/50 p-4">
        <Activity className="size-8 text-text-tertiary" />
      </div>
      <VStack alignItems="center" space={1}>
        <p className="text-sm font-medium text-text">
          No metrics collected yet
        </p>
        <p className="max-w-[320px] text-pretty text-center text-xs text-text-tertiary">
          {description}
        </p>
      </VStack>
    </VStack>
  );
}
