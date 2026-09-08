import { useState } from 'react';
import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { Activity, Cpu, Database, Gauge, HardDrive } from 'lucide-react';
import {
  getNodeMetricsOptions,
  getLatestNodeMetricOptions,
} from '~/queries/nodes';
import { SectionHeader } from '~/components/interface/section-header';
import { VStack } from '~/components/interface/stacks';
import { CHART_COLORS } from '~/components/metrics/chart-theme';
import { MetricChart } from '~/components/metrics/metric-chart';
import { MetricStatCard } from '~/components/metrics/metric-stat-card';
import { MetricsEmptyState } from '~/components/metrics/metrics-empty-state';
import { MetricsLiveBadge } from '~/components/metrics/metrics-live-badge';
import { MetricsSkeleton } from '~/components/metrics/metrics-skeleton';
import { MetricsTimeRange } from '~/components/metrics/metrics-time-range';
import { useMetricsTimeFormat } from '~/hooks/use-metrics-time-format';
import {
  type TimeRange,
  TIME_RANGES,
  formatBps,
  formatMiB,
} from '~/utils/metrics-utils';

type NodeMetricsViewProps = {
  nodeId: string;
};

function percent(used: number | bigint, total: number | bigint) {
  const numTotal = Number(total);
  if (!numTotal) {
    return 0;
  }

  return Math.round((Number(used) / numTotal) * 100);
}

export function NodeMetricsView(props: NodeMetricsViewProps) {
  const { nodeId } = props;
  const [selectedRange, setSelectedRange] = useState<TimeRange>(TIME_RANGES[0]);

  const { data, isLoading } = useQuery({
    ...getNodeMetricsOptions(nodeId, selectedRange.hours),
    placeholderData: keepPreviousData,
  });
  const { data: latestData } = useQuery(getLatestNodeMetricOptions(nodeId));

  const metrics = data?.metrics ?? [];
  const latest = latestData?.metric ?? metrics[metrics.length - 1];
  const formatTimestamp = useMetricsTimeFormat(selectedRange.hours);

  if (isLoading && metrics.length === 0) {
    return <MetricsSkeleton />;
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <SectionHeader
        className="shrink-0"
        icon={Activity}
        title="Node Metrics"
        actions={
          <>
            <MetricsLiveBadge />
            <MetricsTimeRange
              value={selectedRange}
              onChange={setSelectedRange}
              isLoading={isLoading}
            />
          </>
        }
      />

      <div className="custom-scrollbar flex-1 overflow-y-auto p-8">
        {metrics.length === 0 ? (
          <MetricsEmptyState description="Node metrics are collected every 15 seconds. Graphs will begin appearing shortly." />
        ) : (
          <VStack space={6}>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <MetricStatCard
                label="CPU Usage"
                icon={Cpu}
                value={`${latest?.cpu_usage.toFixed(2) ?? '0.00'}%`}
                hint="Host-wide utilization"
              />
              <MetricStatCard
                label="Memory Usage"
                icon={Database}
                value={formatMiB(latest?.memory_used ?? 0)}
                hint={`${percent(latest?.memory_used ?? 0, latest?.memory_total ?? 0)}% of ${formatMiB(latest?.memory_total ?? 0)}`}
              />
              <MetricStatCard
                label="Disk Usage"
                icon={HardDrive}
                value={formatMiB(latest?.disk_used ?? 0)}
                hint={`${percent(latest?.disk_used ?? 0, latest?.disk_total ?? 0)}% of ${formatMiB(latest?.disk_total ?? 0)}`}
              />
              <MetricStatCard
                label="Load Average"
                icon={Gauge}
                value={latest?.load_average.toFixed(2) ?? '0.00'}
                hint="1-minute average"
              />
            </div>

            <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
              <MetricChart
                title="CPU Utilization (%)"
                data={metrics}
                formatTimestamp={formatTimestamp}
                formatValue={(value) => `${value.toFixed(2)}%`}
                formatAxisValue={(value) => `${value}%`}
                series={[
                  {
                    dataKey: 'cpu_usage',
                    label: 'CPU Usage',
                    color: CHART_COLORS.cpu,
                  },
                ]}
              />

              <MetricChart
                title="Memory Utilization"
                data={metrics}
                formatTimestamp={formatTimestamp}
                formatValue={formatMiB}
                series={[
                  {
                    dataKey: 'memory_used',
                    label: 'Memory Used',
                    color: CHART_COLORS.memory,
                  },
                ]}
              />

              <MetricChart
                title="Network I/O Rate"
                variant="line"
                data={metrics}
                formatTimestamp={formatTimestamp}
                formatValue={formatBps}
                series={[
                  {
                    dataKey: 'network_rx_bps',
                    label: 'Receive (RX)',
                    color: CHART_COLORS.networkRx,
                  },
                  {
                    dataKey: 'network_tx_bps',
                    label: 'Transmit (TX)',
                    color: CHART_COLORS.networkTx,
                  },
                ]}
              />

              <MetricChart
                title="Load Average"
                data={metrics}
                formatTimestamp={formatTimestamp}
                formatValue={(value) => value.toFixed(2)}
                formatAxisValue={(value) => value.toFixed(1)}
                series={[
                  {
                    dataKey: 'load_average',
                    label: 'Load (1m)',
                    color: CHART_COLORS.load,
                  },
                ]}
              />
            </div>
          </VStack>
        )}
      </div>
    </div>
  );
}
