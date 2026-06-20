import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Area,
  AreaChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { Activity, Cpu, Database, Globe, HardDrive } from 'lucide-react';
import { getAppMetricsOptions } from '~/queries/apps';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { parseUTC } from '~/utils/format';
import {
  type TimeRange,
  TIME_RANGES,
  formatBytes,
  formatBps,
  formatMiB,
} from '~/components/apps/metrics-utils';

export function AppMetricsView(props: { appSlug: string }) {
  const { appSlug } = props;
  const [selectedRange, setSelectedRange] = useState<TimeRange>(TIME_RANGES[0]);

  const { data, isLoading } = useQuery({
    ...getAppMetricsOptions(appSlug, selectedRange.hours),
    refetchInterval: 10000,
  });

  const rawMetrics = data?.metrics ?? [];

  const metrics = [...rawMetrics].sort(
    (a, b) =>
      parseUTC(a.created_at).getTime() - parseUTC(b.created_at).getTime()
  );

  const formatTime = (isoString: any) => {
    if (!isoString) {
      return '';
    }
    try {
      const d =
        typeof isoString === 'string'
          ? parseUTC(isoString)
          : new Date(isoString);
      if (isNaN(d.getTime())) {
        return '';
      }
      if (selectedRange.hours > 24) {
        return d.toLocaleDateString(undefined, {
          month: 'short',
          day: 'numeric',
          hour: '2-digit',
          minute: '2-digit',
        });
      }
      return d.toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      });
    } catch {
      return '';
    }
  };

  const latest = metrics[metrics.length - 1];

  if (isLoading && metrics.length === 0) {
    return (
      <VStack className="p-8" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-surface-hover" />
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="h-24 animate-pulse rounded-lg border border-border bg-surface"
            />
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
          {[1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="h-72 animate-pulse rounded-lg border border-border bg-surface"
            />
          ))}
        </div>
      </VStack>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <HStack
        justifyContent="between"
        className="border-b border-border px-8 py-4 shrink-0"
      >
        <HStack space={2}>
          <Activity className="size-4 text-text-tertiary" />
          <h2 className="text-sm font-semibold text-text">System Metrics</h2>
          <span
            className="inline-flex h-2 w-2 rounded-full bg-emerald-500 animate-pulse ml-1"
            title="Real-time monitoring active"
          />
        </HStack>
        <HStack
          space={1}
          className="rounded border border-border bg-surface p-0.5"
        >
          {TIME_RANGES.map((range) => (
            <button
              key={range.hours}
              onClick={() => setSelectedRange(range)}
              className={cn(
                'h-7 px-3 rounded text-[11px] font-medium transition-colors',
                selectedRange.hours === range.hours
                  ? 'bg-white/[0.08] text-text'
                  : 'text-text-tertiary hover:text-text'
              )}
            >
              {range.label}
            </button>
          ))}
        </HStack>
      </HStack>

      <div className="flex-1 overflow-y-auto p-8 custom-scrollbar">
        {metrics.length === 0 ? (
          <VStack className="items-center justify-center py-20" space={4}>
            <div className="rounded-full border border-border p-4 bg-surface/50">
              <Activity className="size-8 text-text-tertiary" />
            </div>
            <VStack alignItems="center" space={1}>
              <p className="text-sm font-medium text-text">
                No metrics collected yet
              </p>
              <p className="text-xs text-text-tertiary text-center max-w-[320px]">
                System metrics are collected every 10 seconds. Graphs will begin
                appearing shortly.
              </p>
            </VStack>
          </VStack>
        ) : (
          <VStack space={6}>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>CPU Usage</span>
                  <Cpu className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight">
                  {latest?.cpu_usage.toFixed(2) ?? '0.00'}%
                </div>
                <p className="text-[11px] text-text-tertiary mt-1">
                  Aggregated across all containers
                </p>
              </div>

              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>Memory Usage</span>
                  <Database className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight">
                  {formatMiB(latest?.memory_used ?? 0)}
                </div>
                <p className="text-[11px] text-text-tertiary mt-1">
                  Limit:{' '}
                  {latest?.memory_limit
                    ? formatMiB(latest.memory_limit)
                    : 'Uncapped'}
                </p>
              </div>

              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>Network IO</span>
                  <Globe className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight truncate">
                  {formatBps(latest?.network_rx_bps ?? 0)}
                </div>
                <p className="text-[11px] text-text-tertiary mt-1 flex gap-2">
                  <span>↓ RX: {formatBps(latest?.network_rx_bps ?? 0)}</span>
                  <span>↑ TX: {formatBps(latest?.network_tx_bps ?? 0)}</span>
                </p>
              </div>

              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>Disk IO</span>
                  <HardDrive className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight truncate">
                  {formatBps(
                    (latest?.disk_read_bps ?? 0) + (latest?.disk_write_bps ?? 0)
                  )}
                </div>
                <p className="text-[11px] text-text-tertiary mt-1 flex gap-2">
                  <span>R: {formatBps(latest?.disk_read_bps ?? 0)}</span>
                  <span>W: {formatBps(latest?.disk_write_bps ?? 0)}</span>
                </p>
              </div>
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="rounded-lg border border-border bg-surface p-6">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary mb-4">
                  CPU Utilization (%)
                </h3>
                <div className="h-64 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                      data={metrics}
                      margin={{ top: 10, right: 5, left: -20, bottom: 0 }}
                    >
                      <defs>
                        <linearGradient
                          id="cpuGrad"
                          x1="0"
                          y1="0"
                          x2="0"
                          y2="1"
                        >
                          <stop
                            offset="5%"
                            stopColor="#06b6d4"
                            stopOpacity={0.2}
                          />
                          <stop
                            offset="95%"
                            stopColor="#06b6d4"
                            stopOpacity={0}
                          />
                        </linearGradient>
                      </defs>
                      <CartesianGrid
                        stroke="rgba(255, 255, 255, 0.05)"
                        vertical={false}
                      />
                      <XAxis
                        dataKey="created_at"
                        tickFormatter={formatTime}
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        dy={10}
                      />
                      <YAxis
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        tickFormatter={(v) => `${v}%`}
                        domain={[0, 'auto']}
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: '#232323',
                          borderColor: 'rgba(255, 255, 255, 0.08)',
                          borderRadius: '8px',
                          color: '#fff',
                        }}
                        labelStyle={{
                          color: 'rgba(255, 255, 255, 0.5)',
                          fontSize: '11px',
                        }}
                        itemStyle={{ fontSize: '12px' }}
                        labelFormatter={formatTime}
                        formatter={(value: any) => [
                          `${parseFloat(value).toFixed(3)}%`,
                          'CPU Usage',
                        ]}
                      />
                      <Area
                        type="monotone"
                        dataKey="cpu_usage"
                        stroke="#06b6d4"
                        strokeWidth={1.5}
                        fillOpacity={1}
                        fill="url(#cpuGrad)"
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* Memory Chart */}
              <div className="rounded-lg border border-border bg-surface p-6">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary mb-4">
                  Memory Utilization
                </h3>
                <div className="h-64 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                      data={metrics}
                      margin={{ top: 10, right: 5, left: -20, bottom: 0 }}
                    >
                      <defs>
                        <linearGradient
                          id="memGrad"
                          x1="0"
                          y1="0"
                          x2="0"
                          y2="1"
                        >
                          <stop
                            offset="5%"
                            stopColor="#a855f7"
                            stopOpacity={0.2}
                          />
                          <stop
                            offset="95%"
                            stopColor="#a855f7"
                            stopOpacity={0}
                          />
                        </linearGradient>
                      </defs>
                      <CartesianGrid
                        stroke="rgba(255, 255, 255, 0.05)"
                        vertical={false}
                      />
                      <XAxis
                        dataKey="created_at"
                        tickFormatter={formatTime}
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        dy={10}
                      />
                      <YAxis
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        tickFormatter={(v) => formatMiB(v)}
                        domain={[0, 'auto']}
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: '#232323',
                          borderColor: 'rgba(255, 255, 255, 0.08)',
                          borderRadius: '8px',
                          color: '#fff',
                        }}
                        labelStyle={{
                          color: 'rgba(255, 255, 255, 0.5)',
                          fontSize: '11px',
                        }}
                        itemStyle={{ fontSize: '12px' }}
                        labelFormatter={formatTime}
                        formatter={(value: any) => [
                          formatMiB(value),
                          'Memory Used',
                        ]}
                      />
                      <Area
                        type="monotone"
                        dataKey="memory_used"
                        stroke="#a855f7"
                        strokeWidth={1.5}
                        fillOpacity={1}
                        fill="url(#memGrad)"
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* Network IO Chart */}
              <div className="rounded-lg border border-border bg-surface p-6">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary mb-4">
                  Network I/O Rate
                </h3>
                <div className="h-64 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart
                      data={metrics}
                      margin={{ top: 10, right: 5, left: -20, bottom: 0 }}
                    >
                      <CartesianGrid
                        stroke="rgba(255, 255, 255, 0.05)"
                        vertical={false}
                      />
                      <XAxis
                        dataKey="created_at"
                        tickFormatter={formatTime}
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        dy={10}
                      />
                      <YAxis
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        tickFormatter={(v) => formatBps(v)}
                        domain={[0, 'auto']}
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: '#232323',
                          borderColor: 'rgba(255, 255, 255, 0.08)',
                          borderRadius: '8px',
                          color: '#fff',
                        }}
                        labelStyle={{
                          color: 'rgba(255, 255, 255, 0.5)',
                          fontSize: '11px',
                        }}
                        itemStyle={{ fontSize: '12px' }}
                        labelFormatter={formatTime}
                        formatter={(value: any, name: any) => [
                          formatBps(value),
                          name === 'network_rx_bps'
                            ? 'Receive (RX)'
                            : 'Transmit (TX)',
                        ]}
                      />
                      <Line
                        type="monotone"
                        dataKey="network_rx_bps"
                        name="network_rx_bps"
                        stroke="#3b82f6"
                        strokeWidth={1.5}
                        dot={false}
                        activeDot={{ r: 4 }}
                      />
                      <Line
                        type="monotone"
                        dataKey="network_tx_bps"
                        name="network_tx_bps"
                        stroke="#f97316"
                        strokeWidth={1.5}
                        dot={false}
                        activeDot={{ r: 4 }}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              </div>

              <div className="rounded-lg border border-border bg-surface p-6">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary mb-4">
                  Disk I/O Rate
                </h3>
                <div className="h-64 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart
                      data={metrics}
                      margin={{ top: 10, right: 5, left: -20, bottom: 0 }}
                    >
                      <CartesianGrid
                        stroke="rgba(255, 255, 255, 0.05)"
                        vertical={false}
                      />
                      <XAxis
                        dataKey="created_at"
                        tickFormatter={formatTime}
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        dy={10}
                      />
                      <YAxis
                        stroke="rgba(255, 255, 255, 0.3)"
                        fontSize={10}
                        tickLine={false}
                        tickFormatter={(v) => formatBps(v)}
                        domain={[0, 'auto']}
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: '#232323',
                          borderColor: 'rgba(255, 255, 255, 0.08)',
                          borderRadius: '8px',
                          color: '#fff',
                        }}
                        labelStyle={{
                          color: 'rgba(255, 255, 255, 0.5)',
                          fontSize: '11px',
                        }}
                        itemStyle={{ fontSize: '12px' }}
                        labelFormatter={formatTime}
                        formatter={(value: any, name: any) => [
                          formatBps(value),
                          name === 'disk_read_bps' ? 'Read' : 'Write',
                        ]}
                      />
                      <Line
                        type="monotone"
                        dataKey="disk_read_bps"
                        name="disk_read_bps"
                        stroke="#f43f5e"
                        strokeWidth={1.5}
                        dot={false}
                        activeDot={{ r: 4 }}
                      />
                      <Line
                        type="monotone"
                        dataKey="disk_write_bps"
                        name="disk_write_bps"
                        stroke="#f59e0b"
                        strokeWidth={1.5}
                        dot={false}
                        activeDot={{ r: 4 }}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              </div>
            </div>
          </VStack>
        )}
      </div>
    </div>
  );
}
