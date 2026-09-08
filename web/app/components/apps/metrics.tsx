import { useState } from 'react';
import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { Activity, Cpu, Database, Globe, HardDrive } from 'lucide-react';
import {
  getAppMetricsOptions,
  getLatestAppMetricOptions,
} from '~/queries/apps';
import { SectionHeader } from '~/components/interface/section-header';
import { HStack, VStack } from '~/components/interface/stacks';
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

type AppMetricsViewProps = {
  appSlug: string;
};

export function AppMetricsView(props: AppMetricsViewProps) {
  const { appSlug } = props;
  const [selectedRange, setSelectedRange] = useState<TimeRange>(TIME_RANGES[0]);

  const { data, isLoading } = useQuery({
    ...getAppMetricsOptions(appSlug, selectedRange.hours),
    placeholderData: keepPreviousData,
  });
  const { data: latestData } = useQuery(getLatestAppMetricOptions(appSlug));

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
        title="System Metrics"
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
          <MetricsEmptyState description="System metrics are collected every 10 seconds. Graphs will begin appearing shortly." />
        ) : (
          <VStack space={6}>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <MetricStatCard
                label="CPU Usage"
                icon={Cpu}
                value={`${latest?.cpu_usage.toFixed(2) ?? '0.00'}%`}
                hint="Aggregated across all containers"
              />
              <MetricStatCard
                label="Memory Usage"
                icon={Database}
                value={formatMiB(latest?.memory_used ?? 0)}
                hint={`Limit: ${latest?.memory_limit ? formatMiB(latest.memory_limit) : 'Uncapped'}`}
              />
              <MetricStatCard
                label="Network IO"
                icon={Globe}
                value={formatBps(latest?.network_rx_bps ?? 0)}
                hint={
                  <HStack space={2}>
                    <span>↓ RX: {formatBps(latest?.network_rx_bps ?? 0)}</span>
                    <span>↑ TX: {formatBps(latest?.network_tx_bps ?? 0)}</span>
                  </HStack>
                }
              />
              <MetricStatCard
                label="Disk IO"
                icon={HardDrive}
                value={formatBps(
                  (latest?.disk_read_bps ?? 0) + (latest?.disk_write_bps ?? 0)
                )}
                hint={
                  <HStack space={2}>
                    <span>R: {formatBps(latest?.disk_read_bps ?? 0)}</span>
                    <span>W: {formatBps(latest?.disk_write_bps ?? 0)}</span>
                  </HStack>
                }
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
                title="Disk I/O Rate"
                variant="line"
                data={metrics}
                formatTimestamp={formatTimestamp}
                formatValue={formatBps}
                series={[
                  {
                    dataKey: 'disk_read_bps',
                    label: 'Read',
                    color: CHART_COLORS.diskRead,
                  },
                  {
                    dataKey: 'disk_write_bps',
                    label: 'Write',
                    color: CHART_COLORS.diskWrite,
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
