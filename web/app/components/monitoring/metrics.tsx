import { useState, useMemo, useCallback } from 'react';
import { useQuery, keepPreviousData } from '@tanstack/react-query';
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
import { Activity, Cpu, Database, Gauge, HardDrive } from 'lucide-react';
import {
  getServerMetricsOptions,
  getLatestServerMetricOptions,
} from '~/queries/monitoring';
import { SectionHeader } from '~/components/interface/section-header';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { parseUTC } from '~/utils/format';
import {
  type TimeRange,
  TIME_RANGES,
  formatBps,
  formatMiB,
} from '~/components/apps/metrics-utils';

const percent = (used: number | bigint, total: number | bigint) => {
  const numTotal = Number(total);
  if (!numTotal) {
    return 0;
  }
  return Math.round((Number(used) / numTotal) * 100);
};

export function ServerMetricsView() {
  const [selectedRange, setSelectedRange] = useState<TimeRange>(TIME_RANGES[0]);

  const { data, isLoading } = useQuery({
    ...getServerMetricsOptions(selectedRange.hours),
    placeholderData: keepPreviousData,
  });

  const { data: latestData } = useQuery(getLatestServerMetricOptions());

  const metrics = useMemo(() => data?.metrics ?? [], [data]);

  const formatTime = useCallback(
    (isoString: any) => {
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
    },
    [selectedRange.hours]
  );

  const latest = latestData?.metric ?? metrics[metrics.length - 1];

  if (isLoading && metrics.length === 0) {
    return (
      <VStack className="p-8" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-white/[0.06]" />
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
      <SectionHeader
        className="shrink-0"
        icon={Activity}
        title="Server Metrics"
        actions={
          <>
            <HStack space={1.5} alignItems="center">
              <span
                className="inline-flex size-2 rounded-full bg-emerald-500 animate-pulse"
                title="Real-time monitoring active"
              />
              <span className="text-[11px] font-medium text-text-tertiary">
                Live
              </span>
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
          </>
        }
      />

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
                Server metrics are collected every 15 seconds. Graphs will begin
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
                  Host-wide utilization
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
                  {percent(latest?.memory_used ?? 0, latest?.memory_total ?? 0)}
                  % of {formatMiB(latest?.memory_total ?? 0)}
                </p>
              </div>

              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>Disk Usage</span>
                  <HardDrive className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight">
                  {formatMiB(latest?.disk_used ?? 0)}
                </div>
                <p className="text-[11px] text-text-tertiary mt-1">
                  {percent(latest?.disk_used ?? 0, latest?.disk_total ?? 0)}% of{' '}
                  {formatMiB(latest?.disk_total ?? 0)}
                </p>
              </div>

              <div className="rounded-lg border border-border bg-surface p-4">
                <HStack
                  justifyContent="between"
                  className="text-text-tertiary text-xs font-medium"
                >
                  <span>Load Average</span>
                  <Gauge className="size-4" />
                </HStack>
                <div className="mt-2 text-2xl font-semibold text-text tracking-tight">
                  {latest?.load_average.toFixed(2) ?? '0.00'}
                </div>
                <p className="text-[11px] text-text-tertiary mt-1">
                  1-minute average
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
                          id="srvCpuGrad"
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
                          `${parseFloat(value).toFixed(2)}%`,
                          'CPU Usage',
                        ]}
                      />
                      <Area
                        type="monotone"
                        dataKey="cpu_usage"
                        stroke="#06b6d4"
                        strokeWidth={1.5}
                        fillOpacity={1}
                        fill="url(#srvCpuGrad)"
                        isAnimationActive={false}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

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
                          id="srvMemGrad"
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
                        fill="url(#srvMemGrad)"
                        isAnimationActive={false}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

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
                        isAnimationActive={false}
                      />
                      <Line
                        type="monotone"
                        dataKey="network_tx_bps"
                        name="network_tx_bps"
                        stroke="#f97316"
                        strokeWidth={1.5}
                        dot={false}
                        activeDot={{ r: 4 }}
                        isAnimationActive={false}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              </div>

              <div className="rounded-lg border border-border bg-surface p-6">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary mb-4">
                  Load Average
                </h3>
                <div className="h-64 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                      data={metrics}
                      margin={{ top: 10, right: 5, left: -20, bottom: 0 }}
                    >
                      <defs>
                        <linearGradient
                          id="srvLoadGrad"
                          x1="0"
                          y1="0"
                          x2="0"
                          y2="1"
                        >
                          <stop
                            offset="5%"
                            stopColor="#22c55e"
                            stopOpacity={0.2}
                          />
                          <stop
                            offset="95%"
                            stopColor="#22c55e"
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
                        tickFormatter={(v) => v.toFixed(1)}
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
                          parseFloat(value).toFixed(2),
                          'Load (1m)',
                        ]}
                      />
                      <Area
                        type="monotone"
                        dataKey="load_average"
                        stroke="#22c55e"
                        strokeWidth={1.5}
                        fillOpacity={1}
                        fill="url(#srvLoadGrad)"
                        isAnimationActive={false}
                      />
                    </AreaChart>
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
