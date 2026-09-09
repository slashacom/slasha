import { VStack } from '~/components/interface/stacks';

export function MetricsSkeleton() {
  return (
    <VStack className="px-8 py-6" space={4}>
      <div className="h-4 w-32 animate-pulse rounded bg-white/[0.06]" />
      <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
        {[1, 2, 3, 4].map((item) => (
          <div
            key={item}
            className="h-24 animate-pulse rounded-lg border border-border bg-surface"
          />
        ))}
      </div>
      <div className="mt-4 grid grid-cols-1 gap-6 lg:grid-cols-2">
        {[1, 2, 3, 4].map((item) => (
          <div
            key={item}
            className="h-72 animate-pulse rounded-lg border border-border bg-surface"
          />
        ))}
      </div>
    </VStack>
  );
}
