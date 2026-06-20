import { CircleDashed } from 'lucide-react';
import type { ProcessContainer } from '~/models/app-scale';
import type { DeploymentStatus } from '~/models/deployment';
import { VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type ProcessExplorerProps = {
  processes: ProcessContainer[];
  deploymentStatus: DeploymentStatus;
};

export function ProcessExplorer(props: ProcessExplorerProps) {
  const { processes, deploymentStatus } = props;
  const isProvisioning =
    deploymentStatus === 'Pending' || deploymentStatus === 'Building';

  return (
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
            <th className="px-6 py-2.5 text-right text-[11px] font-semibold uppercase tracking-wider text-text-tertiary">
              Status
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-border">
          {processes.length === 0 ? (
            <tr>
              <td colSpan={4} className="px-6 py-12 text-center">
                {isProvisioning ? (
                  <VStack space={2} alignItems="center">
                    <CircleDashed className="size-5 animate-spin text-text-tertiary" />
                    <p className="text-xs text-text-tertiary">
                      Initializing processes...
                    </p>
                  </VStack>
                ) : (
                  <p className="text-xs text-text-tertiary">
                    {deploymentStatus === 'Failed'
                      ? 'This deployment failed — no processes are running.'
                      : deploymentStatus === 'Stopped'
                        ? 'This deployment is stopped.'
                        : 'No processes are running.'}
                  </p>
                )}
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
  );
}
