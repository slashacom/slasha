import { Boxes, CircleDashed } from 'lucide-react';
import type { ProcessContainer } from '~/models/app-scale';
import type { DeploymentStatus } from '~/models/deployment';
import { EmptyPage } from '~/components/global/empty-page';
import { HStack, VStack } from '~/components/interface/stacks';
import { Table } from '~/components/interface/table';
import { cn } from '~/utils/classname';

type ProcessExplorerProps = {
  processes: ProcessContainer[];
  deploymentStatus: DeploymentStatus;
};

function emptySubtitle(deploymentStatus: DeploymentStatus) {
  if (deploymentStatus === 'Failed') {
    return 'This deployment failed, so nothing is running. Check the deployment logs, then deploy again.';
  }

  if (deploymentStatus === 'Stopped') {
    return 'This deployment is stopped. Start it again to bring its containers back.';
  }

  return 'Containers appear here as soon as the deployment starts them.';
}

const TYPE_ORDER: Record<string, number> = { web: 0, worker: 1, release: 2 };

export function ProcessExplorer(props: ProcessExplorerProps) {
  const { processes, deploymentStatus } = props;
  const ordered = [...processes].sort((a, b) => {
    const byType =
      (TYPE_ORDER[a.process_type] ?? 99) - (TYPE_ORDER[b.process_type] ?? 99);
    if (byType !== 0) {
      return byType;
    }

    return a.instance_index - b.instance_index;
  });
  const isProvisioning =
    deploymentStatus === 'Pending' || deploymentStatus === 'Building';

  if (isProvisioning) {
    return (
      <VStack
        space={2}
        alignItems="center"
        className="rounded-lg border border-dashed border-border bg-surface/30 px-6 py-14"
      >
        <CircleDashed className="size-5 animate-spin text-text-tertiary" />
        <p className="text-[13px] text-text-tertiary">
          Starting containers for this deployment.
        </p>
      </VStack>
    );
  }

  if (processes.length === 0) {
    return (
      <EmptyPage
        icon={Boxes}
        title="No processes running."
        subtitle={emptySubtitle(deploymentStatus)}
      />
    );
  }

  return (
    <div className="-mx-8 overflow-x-auto">
      <Table
        columns={[
          'Process',
          'Instance',
          'Container',
          { label: 'Status', align: 'right' },
        ]}
      >
        {ordered.map((process) => (
          <tr key={process.name}>
            <td className="py-3 pr-4 text-[13px] font-medium text-text">
              {process.process_type}
            </td>
            <td className="py-3 pr-4 font-mono text-[12px] text-text-secondary">
              #{process.instance_index}
            </td>
            <td className="py-3 pr-4 font-mono text-[12px] text-text-tertiary">
              {process.name}
            </td>
            <td className="py-3 text-right">
              <HStack space={1.5} justifyContent="end">
                <span
                  className={cn(
                    'size-1.5 rounded-full',
                    process.status === 'Running'
                      ? 'bg-emerald-400'
                      : 'bg-text-tertiary'
                  )}
                />
                <span className="text-[12px] text-text-secondary">
                  {process.status}
                </span>
              </HStack>
            </td>
          </tr>
        ))}
      </Table>
    </div>
  );
}
