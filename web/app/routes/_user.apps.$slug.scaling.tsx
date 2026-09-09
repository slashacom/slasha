import { useParams } from 'react-router';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Layers } from 'lucide-react';
import {
  getDeploymentsOptions,
  getProcessesOptions,
} from '~/queries/deployments';
import { getScalesOptions } from '~/queries/apps';
import type { ProcessType } from '~/models/app-scale';
import { EmptyPage } from '~/components/global/empty-page';
import { VStack } from '~/components/interface/stacks';
import { ScaleCard } from '~/components/apps/scale-card';
import { ProcessExplorer } from '~/components/apps/process-explorer';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getScalesOptions(params.slug)),
    queryClient.ensureQueryData(getDeploymentsOptions(params.slug)),
  ]);
}

export default function AppScalingPage() {
  const { slug } = useParams();

  const { data: deploymentsData } = useSuspenseQuery(
    getDeploymentsOptions(slug!)
  );
  const { data: scalesData } = useSuspenseQuery({
    ...getScalesOptions(slug!),
    refetchInterval: 2000,
  });

  const runningDeployment = deploymentsData.deployments.find(
    (d) => d.status === 'Running'
  );
  const scales = scalesData.scales ?? [];

  const { data: processesData } = useQuery({
    ...getProcessesOptions(slug!, runningDeployment?.id ?? ''),
    enabled: !!runningDeployment,
    refetchInterval: 2000,
  });
  const processes = processesData?.processes ?? [];

  if (!runningDeployment) {
    return (
      <EmptyPage
        className="mx-8 my-6 flex-1"
        icon={Layers}
        size="lg"
        title="App is not running."
        subtitle="Scaling controls unlock once a deployment is live. Deploy the app to manage process replicas."
        actionLabel="View deployments"
        actionColor="neutral"
        actionTo={`/apps/${slug}/deployments`}
      />
    );
  }

  const processGroups: Record<string, number> = {};
  for (const p of processes) {
    if (p.process_type !== 'release') {
      processGroups[p.process_type] =
        (processGroups[p.process_type] ?? 0) + (p.status === 'Running' ? 1 : 0);
    }
  }
  for (const s of scales) {
    if (s.process_type !== 'release' && !(s.process_type in processGroups)) {
      processGroups[s.process_type] = 0;
    }
  }
  if (Object.keys(processGroups).length === 0) {
    processGroups.web = 0;
  }

  const typeOrder: Record<string, number> = { web: 0, worker: 1, release: 2 };
  const scalableTypes = (Object.keys(processGroups) as ProcessType[]).sort(
    (a, b) => (typeOrder[a] ?? 99) - (typeOrder[b] ?? 99)
  );

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <div className="px-8 py-6">
        <VStack space={6}>
          <VStack space={3}>
            <VStack space={1}>
              <h3 className="text-sm font-semibold text-text">Replicas</h3>
              <p className="text-[12px] text-text-tertiary">
                Number of containers to run for each process type.
              </p>
            </VStack>
            <div className="grid gap-4 [grid-template-columns:repeat(auto-fill,minmax(15rem,18rem))]">
              {scalableTypes.map((type) => (
                <ScaleCard
                  key={type}
                  appSlug={slug!}
                  deploymentId={runningDeployment.id}
                  processType={type}
                  desiredCount={
                    scales.find((s) => s.process_type === type)?.desired ??
                    Math.max(1, processGroups[type])
                  }
                  runningCount={processGroups[type]}
                />
              ))}
            </div>
          </VStack>

          <VStack space={3}>
            <VStack space={1}>
              <h3 className="text-sm font-semibold text-text">Processes</h3>
              <p className="text-[12px] text-text-tertiary">
                Containers running for the current deployment.
              </p>
            </VStack>
            <ProcessExplorer
              processes={processes}
              deploymentStatus={runningDeployment.status}
            />
          </VStack>
        </VStack>
      </div>
    </div>
  );
}
