import { useEffect, useState } from 'react';
import { Minus, Plus } from 'lucide-react';
import { toast } from 'sonner';
import type { ProcessType } from '~/models/app-scale';
import { useScaleDeployment } from '~/queries/deployments';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type ScaleCardProps = {
  appSlug: string;
  deploymentId: string;
  processType: ProcessType;
  desiredCount: number;
  runningCount: number;
};

export function ScaleCard(props: ScaleCardProps) {
  const { appSlug, deploymentId, processType, desiredCount, runningCount } =
    props;
  const [count, setCount] = useState(desiredCount);
  const scale = useScaleDeployment();

  useEffect(() => {
    setCount(desiredCount);
  }, [desiredCount]);

  const isHealthy = runningCount === desiredCount && runningCount > 0;
  const isDirty = count !== desiredCount;

  const handleScale = async () => {
    if (count < 1) {
      toast.error('Scale must be at least 1');
      return;
    }
    try {
      await scale.mutateAsync({ appSlug, deploymentId, processType, count });
      toast.success(`Scaling ${processType} to ${count} replica(s)`);
    } catch (e) {
      toast.error('Failed to scale: ' + e);
    }
  };

  return (
    <div className="rounded-lg border border-border bg-surface p-5">
      <VStack space={4}>
        <HStack justifyContent="between" alignItems="center">
          <HStack space={2} alignItems="center">
            <div
              className={cn(
                'size-2 rounded-full',
                isHealthy
                  ? 'bg-emerald-500 shadow-[0_0_5px_rgba(16,185,129,0.5)]'
                  : runningCount > 0
                    ? 'bg-amber-500 animate-pulse'
                    : 'bg-text-tertiary'
              )}
            />
            <h4 className="text-[13px] font-semibold uppercase tracking-wide text-text">
              {processType}
            </h4>
          </HStack>
          <span className="text-[11px] font-medium text-text-tertiary">
            {runningCount} / {desiredCount} up
          </span>
        </HStack>

        <HStack space={3} alignItems="center">
          <div className="flex h-9 flex-1 items-center rounded-md border border-border bg-bg">
            <button
              type="button"
              aria-label="Decrease"
              onClick={() => setCount((c) => Math.max(1, c - 1))}
              disabled={count <= 1}
              className="flex h-full w-9 items-center justify-center text-text-tertiary transition-colors hover:text-text disabled:opacity-40"
            >
              <Minus className="size-3.5" />
            </button>
            <div className="flex-1 text-center font-mono text-sm font-semibold text-text tabular-nums">
              {count}
            </div>
            <button
              type="button"
              aria-label="Increase"
              onClick={() => setCount((c) => c + 1)}
              className="flex h-full w-9 items-center justify-center text-text-tertiary transition-colors hover:text-text"
            >
              <Plus className="size-3.5" />
            </button>
          </div>
          <Button
            label="Apply"
            size="sm"
            variant={isDirty ? 'default' : 'ghost'}
            onClick={handleScale}
            isLoading={scale.isPending}
            isDisabled={!isDirty}
          />
        </HStack>
      </VStack>
    </div>
  );
}
