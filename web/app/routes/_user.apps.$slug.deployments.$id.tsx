import { useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import {
  ArrowLeft,
  Box,
  ChevronRight,
  CircleDashed,
  History,
  Layers,
  Plus,
  Minus,
} from 'lucide-react';
import {
  getDeploymentOptions,
  getProcessesOptions,
  useScaleDeployment,
} from '~/queries/deployments';
import { getAppOptions, getScalesOptions } from '~/queries/apps';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({
  params,
}: {
  params: { slug: string; id: string };
}) {
  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getDeploymentOptions(params.slug, params.id)),
    queryClient.ensureQueryData(getProcessesOptions(params.slug, params.id)),
    queryClient.ensureQueryData(getScalesOptions(params.slug)),
  ]);
}

export default function DeploymentDetailPage() {
  const { slug, id } = useParams();
  const navigate = useNavigate();

  const { data: appData } = useQuery(getAppOptions(slug!));
  const { data: deploymentData } = useQuery(getDeploymentOptions(slug!, id!));
  const { data: processesData } = useQuery(getProcessesOptions(slug!, id!));
  const { data: scalesData } = useQuery(getScalesOptions(slug!));

  const app = appData?.app;
  const deployment = deploymentData?.deployment;
  const processes = processesData?.processes ?? [];
  const scales = scalesData?.scales ?? [];

  if (!app || !deployment) {
    return (
      <div className="flex flex-1 items-center justify-center p-20">
        <CircleDashed className="size-6 animate-spin text-text-tertiary" />
      </div>
    );
  }

  // Group processes by type
  const processGroups = processes.reduce(
    (acc, p) => {
      if (!acc[p.process_type]) acc[p.process_type] = [];
      acc[p.process_type].push(p);
      return acc;
    },
    {} as Record<string, typeof processes>
  );

  // Also include process types from scale configs even if no containers are running
  scales.forEach((s) => {
    if (!processGroups[s.process_type]) {
      processGroups[s.process_type] = [];
    }
  });

  return (
    <div className="flex flex-1 flex-col min-h-0 bg-bg">
      {/* Header Toolbar */}
      <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-8 py-3 bg-surface/30">
        <HStack space={3} alignItems="center">
          <button
            onClick={() => navigate(`/apps/${slug}`)}
            className="group flex h-7 w-7 items-center justify-center rounded border border-border bg-surface transition-all hover:bg-surface-hover"
          >
            <ArrowLeft className="size-3.5 text-text-tertiary group-hover:text-text" />
          </button>
          <HStack space={2} alignItems="center">
            <span className="text-[13px] font-medium text-text">
              {app.name}
            </span>
            <ChevronRight className="size-3 text-text-quaternary" />
            <span className="font-mono text-[12px] text-text-tertiary">
              {deployment.commit_sha.slice(0, 7)}
            </span>
          </HStack>
        </HStack>

        <HStack space={3} alignItems="center">
          <div className="flex items-center gap-2 rounded border border-border bg-surface px-2 py-0.5">
            <div
              className={cn(
                'size-1.5 rounded-full',
                deployment.status === 'Running'
                  ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.3)]'
                  : deployment.status === 'Building'
                    ? 'bg-sky-500 animate-pulse'
                    : 'bg-text-tertiary'
              )}
            />
            <span className="text-[11px] font-medium text-text-secondary">
              {deployment.status}
            </span>
          </div>
          <span className="text-[11px] text-text-tertiary">
            Deployed {formatRelativeTime(deployment.created_at)}
          </span>
        </HStack>
      </div>

      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-5xl space-y-10">
          {/* Scaling Section */}
          <section>
            <HStack space={2} alignItems="center" className="mb-5">
              <Layers className="size-4 text-text-tertiary" />
              <h3 className="text-sm font-semibold text-text">Scaling</h3>
            </HStack>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {Object.keys(processGroups)
                .filter((type) => type !== 'Release')
                .map((type) => (
                  <ScaleCard
                    key={type}
                    appSlug={slug!}
                    deploymentId={id!}
                    processType={type}
                    desiredCount={
                      scales.find((s) => s.process_type === type)?.desired ?? 1
                    }
                    runningCount={
                      processGroups[type].filter((p) => p.status === 'Running')
                        .length
                    }
                  />
                ))}
            </div>
          </section>

          {/* Processes Explorer */}
          <section>
            <HStack space={2} alignItems="center" className="mb-5">
              <Box className="size-4 text-text-tertiary" />
              <h3 className="text-sm font-semibold text-text">
                Process Explorer
              </h3>
            </HStack>
            <div className="overflow-hidden rounded-md border border-border bg-surface/20">
              <table className="w-full text-left">
                <thead>
                  <tr className="border-b border-border bg-surface/50">
                    <th className="px-6 py-2.5 text-[11px] font-semibold uppercase tracking-wider text-text-tertiary">
                      Process Type
                    </th>
                    <th className="px-6 py-2.5 text-[11px] font-semibold uppercase tracking-wider text-text-tertiary">
                      Instance
                    </th>
                    <th className="px-6 py-2.5 text-[11px] font-semibold uppercase tracking-wider text-text-tertiary">
                      Container Identifier
                    </th>
                    <th className="px-6 py-2.5 text-[11px] font-semibold uppercase tracking-wider text-text-tertiary text-right">
                      Status
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {processes.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="px-6 py-12 text-center">
                        <VStack space={2} alignItems="center">
                          <CircleDashed className="size-5 animate-spin text-text-tertiary" />
                          <p className="text-xs text-text-tertiary">
                            Initializing processes...
                          </p>
                        </VStack>
                      </td>
                    </tr>
                  ) : (
                    processes.map((p) => (
                      <tr
                        key={p.name}
                        className="transition-colors hover:bg-white/[0.01]"
                      >
                        <td className="px-6 py-3.5">
                          <span className="text-[13px] font-medium text-text">
                            {p.process_type}
                          </span>
                        </td>
                        <td className="px-6 py-3.5">
                          <span className="font-mono text-[12px] text-text-secondary">
                            #{p.instance_index}
                          </span>
                        </td>
                        <td className="px-6 py-3.5">
                          <span className="font-mono text-[12px] text-text-tertiary">
                            {p.name}
                          </span>
                        </td>
                        <td className="px-6 py-3.5 text-right">
                          <span
                            className={cn(
                              'inline-flex items-center gap-1.5 rounded px-2 py-0.5 text-[11px] font-medium',
                              p.status === 'Running'
                                ? 'bg-emerald-500/10 text-emerald-400'
                                : 'bg-white/5 text-text-tertiary'
                            )}
                          >
                            <div
                              className={cn(
                                'size-1 rounded-full',
                                p.status === 'Running'
                                  ? 'bg-emerald-500'
                                  : 'bg-text-tertiary'
                              )}
                            />
                            {p.status}
                          </span>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function ScaleCard({
  appSlug,
  deploymentId,
  processType,
  desiredCount,
  runningCount,
}: {
  appSlug: string;
  deploymentId: string;
  processType: string;
  desiredCount: number;
  runningCount: number;
}) {
  const [countStr, setCountStr] = useState(desiredCount.toString());
  const scale = useScaleDeployment();

  const count = parseInt(countStr) || 0;

  const handleScale = async () => {
    if (count < 1) {
      toast.error('Scale must be at least 1');
      return;
    }
    try {
      await scale.mutateAsync({
        appSlug,
        deploymentId,
        processType: processType as any,
        count,
      });
      toast.success(`Scaling ${processType} to ${count}`);
    } catch (e) {
      toast.error('Failed to scale: ' + e);
    }
  };

  return (
    <div className="rounded-md border border-border bg-surface p-4 transition-colors hover:border-text-tertiary/20">
      <VStack space={4}>
        <HStack justifyContent="between" alignItems="start">
          <VStack space={0.5}>
            <h4 className="text-[13px] font-bold text-text uppercase tracking-tight">
              {processType}
            </h4>
            <span className="text-[11px] font-medium text-text-tertiary">
              {runningCount} / {desiredCount} replicas up
            </span>
          </VStack>
          <div
            className={cn(
              'size-2 rounded-full',
              runningCount === desiredCount && runningCount > 0
                ? 'bg-emerald-500 shadow-[0_0_5px_rgba(16,185,129,0.5)]'
                : 'bg-primary animate-pulse'
            )}
          />
        </HStack>

        <HStack space={2}>
          <div className="relative flex-1">
            <Input
              value={countStr}
              onChange={(e) => {
                const val = e.target.value;
                if (val === '' || /^\d+$/.test(val)) {
                  setCountStr(val);
                }
              }}
              onBlur={() => {
                if (countStr === '' || parseInt(countStr) < 1) {
                  setCountStr('1');
                }
              }}
              className="h-8 pr-8 font-mono text-[12px] font-semibold bg-bg"
              placeholder="1"
            />
            <div className="absolute right-1 top-1/2 -translate-y-1/2 flex flex-col items-center">
              <button
                onClick={() =>
                  setCountStr((prev) => ((parseInt(prev) || 0) + 1).toString())
                }
                className="p-0.5 hover:text-text text-text-tertiary transition-colors"
              >
                <Plus className="size-2.5" />
              </button>
              <button
                onClick={() =>
                  setCountStr((prev) =>
                    Math.max(1, (parseInt(prev) || 0) - 1).toString()
                  )
                }
                className="p-0.5 hover:text-text text-text-tertiary transition-colors"
              >
                <Minus className="size-2.5" />
              </button>
            </div>
          </div>
          <Button
            label="Scale"
            size="sm"
            variant={count !== desiredCount ? 'default' : 'ghost'}
            onClick={handleScale}
            isLoading={scale.isPending}
            isDisabled={count === desiredCount || !countStr}
          />
        </HStack>
      </VStack>
    </div>
  );
}
