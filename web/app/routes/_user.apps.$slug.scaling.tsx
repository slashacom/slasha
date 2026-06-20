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
import { SectionHeader } from '~/components/interface/section-header';
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
  const { data: scalesData } = useSuspenseQuery(getScalesOptions(slug!));

  const runningDeployment = deploymentsData.deployments.find(
    (d) => d.status === 'Running'
  );
  const scales = scalesData.scales ?? [];

  const { data: processesData } = useQuery({
    ...getProcessesOptions(slug!, runningDeployment?.id ?? ''),
    enabled: !!runningDeployment,
    refetchInterval: 5000,
  });
  const processes = processesData?.processes ?? [];

  if (!runningDeployment) {
    return (
      <EmptyPage
        className="flex-1"
        icon={Layers}
        title="App is not running"
        subtitle="Scaling controls become available once a deployment is running. Deploy your app to manage process replicas."
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

  const scalableTypes = Object.keys(processGroups) as ProcessType[];

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <SectionHeader icon={Layers} title="Scaling" />

      <div className="p-8">
        <VStack space={6}>
          <VStack space={3}>
            <VStack space={1}>
              <h3 className="text-sm font-semibold text-text">Replicas</h3>
              <p className="text-[12px] text-text-tertiary">
                How many containers run for each process type.{' '}
                <span className="font-mono text-text-secondary">web</span>{' '}
                serves HTTP traffic;{' '}
                <span className="font-mono text-text-secondary">worker</span>{' '}
                runs background jobs.
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
              <h3 className="text-sm font-semibold text-text">
                Process Explorer
              </h3>
              <p className="text-[12px] text-text-tertiary">
                Live containers for the running deployment.
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
