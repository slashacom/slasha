import { Clock, Cpu, HardDrive, MemoryStick } from 'lucide-react';
import type { Service } from '~/models/service';
import type { ServiceStats } from '~/queries/services';
import { HStack } from '~/components/interface/stacks';
import { formatFileSize, formatUptime } from '~/utils/format';
import { cn } from '~/utils/classname';

type StatTileProps = {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
  fraction?: number | null;
};

function StatTile(props: StatTileProps) {
  const { icon: Icon, label, value, fraction = null } = props;
  return (
    <div className="rounded-xl border border-border bg-surface/50 px-4 py-4 transition-colors hover:border-white/15">
      <HStack space={1.5} alignItems="center" className="text-text-tertiary">
        <Icon className="size-3.5" />
        <span className="text-[10px] font-medium uppercase tracking-wider">
          {label}
        </span>
      </HStack>
      <div className="mt-2 font-mono text-[22px] font-medium leading-none tracking-tight text-text">
        {value}
      </div>
      {fraction != null ? (
        <HStack space={2} alignItems="center" className="mt-2.5">
          <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-white/10">
            <div
              className={cn(
                'h-full rounded-full',
                fraction >= 0.95
                  ? 'bg-red-500'
                  : fraction >= 0.8
                    ? 'bg-amber-400'
                    : 'bg-emerald-400/80'
              )}
              style={{
                width: `${Math.min(100, Math.max(2, fraction * 100))}%`,
              }}
            />
          </div>
          <span className="text-[10px] tabular-nums text-text-tertiary">
            {Math.round(fraction * 100)}%
          </span>
        </HStack>
      ) : null}
    </div>
  );
}

type ServiceStatsBarProps = {
  service: Service;
  stats?: ServiceStats;
};

export function ServiceStatsBar(props: ServiceStatsBarProps) {
  const { service, stats } = props;
  const isRunning = service.status === 'Running';

  // Only frame memory as used/limit when the service has an explicit cap;
  // otherwise the "limit" is the whole host's RAM, which is misleading.
  const memUsed = stats?.memory_used_bytes ?? null;
  const memLimit =
    service.resources?.memory_bytes != null
      ? Number(service.resources.memory_bytes)
      : null;
  const memValue =
    memUsed == null
      ? '—'
      : memLimit != null
        ? `${formatFileSize(memUsed)} / ${formatFileSize(memLimit)}`
        : formatFileSize(memUsed);
  const memFraction =
    memUsed != null && memLimit != null && memLimit > 0
      ? memUsed / memLimit
      : null;

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
      <StatTile
        icon={Clock}
        label="Uptime"
        value={
          isRunning && stats?.started_at ? formatUptime(stats.started_at) : '—'
        }
      />
      <StatTile
        icon={Cpu}
        label="CPU"
        value={
          stats?.cpu_percent != null ? `${stats.cpu_percent.toFixed(1)}%` : '—'
        }
      />
      <StatTile
        icon={MemoryStick}
        label="Memory"
        value={memValue}
        fraction={memFraction}
      />
      <StatTile
        icon={HardDrive}
        label="Disk"
        value={
          stats?.disk_bytes != null ? formatFileSize(stats.disk_bytes) : '—'
        }
      />
    </div>
  );
}
