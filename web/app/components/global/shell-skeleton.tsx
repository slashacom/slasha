import { PageSkeleton } from '~/components/global/page-skeleton';
import { Skeleton } from '~/components/interface/skeleton';
import { VStack } from '~/components/interface/stacks';

export function ShellSkeleton() {
  return (
    <div className="flex h-screen bg-bg" aria-hidden>
      <aside className="fixed inset-y-0 left-0 z-50 flex w-[240px] flex-col border-r border-border bg-bg">
        <div className="flex h-12 items-center border-b border-border px-6">
          <span className="text-[18px] font-medium tracking-tight text-text">
            slasha
          </span>
        </div>
        <VStack space={3} className="px-6 pt-6">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-24 opacity-80" />
          <Skeleton className="h-4 w-20 opacity-60" />
          <Skeleton className="h-4 w-16 opacity-40" />
        </VStack>
      </aside>

      <div className="ml-[240px] flex flex-1 flex-col overflow-hidden">
        <header className="flex h-12 shrink-0 items-center border-b border-border px-8">
          <Skeleton className="h-4 w-40" />
        </header>
        <PageSkeleton />
      </div>
    </div>
  );
}
