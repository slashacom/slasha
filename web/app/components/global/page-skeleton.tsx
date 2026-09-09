import { Skeleton } from '~/components/interface/skeleton';
import { VStack } from '~/components/interface/stacks';

export function PageSkeleton() {
  return (
    <VStack space={4} className="px-8 py-6" aria-hidden>
      <Skeleton className="h-5 w-48" />
      <VStack space={2}>
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full opacity-80" />
        <Skeleton className="h-12 w-full opacity-60" />
        <Skeleton className="h-12 w-full opacity-40" />
      </VStack>
    </VStack>
  );
}
