import { HStack } from '~/components/interface/stacks';

export function MetricsLiveBadge() {
  return (
    <HStack space={1.5} alignItems="center">
      <span
        className="inline-flex size-2 animate-pulse rounded-full bg-emerald-500"
        title="Real-time monitoring active"
      />
      <span className="text-[11px] font-medium text-text-tertiary">Live</span>
    </HStack>
  );
}
