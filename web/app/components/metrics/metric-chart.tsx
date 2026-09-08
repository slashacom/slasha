import { useId } from 'react';
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
import {
  AXIS_PROPS,
  CHART_MARGIN,
  GRID_STROKE,
  TOOLTIP_PROPS,
} from './chart-theme';

export type MetricSeries = {
  dataKey: string;
  label: string;
  color: string;
};

type MetricChartProps = {
  title: string;
  data: Array<Record<string, unknown>>;
  series: MetricSeries[];
  formatValue: (value: number) => string;
  formatAxisValue?: (value: number) => string;
  formatTimestamp: (value: unknown) => string;
  variant?: 'area' | 'line';
};

export function MetricChart(props: MetricChartProps) {
  const {
    title,
    data,
    series,
    formatValue,
    formatAxisValue = formatValue,
    formatTimestamp,
    variant = 'area',
  } = props;
  const gradientId = useId();
  const labels = new Map(series.map((item) => [item.dataKey, item.label]));

  const axes = (
    <>
      <CartesianGrid stroke={GRID_STROKE} vertical={false} />
      <XAxis
        dataKey="created_at"
        tickFormatter={formatTimestamp}
        dy={10}
        {...AXIS_PROPS}
      />
      <YAxis
        tickFormatter={(value) => formatAxisValue(Number(value))}
        domain={[0, 'auto']}
        {...AXIS_PROPS}
      />
      <Tooltip
        labelFormatter={formatTimestamp}
        formatter={(value, name) => [
          formatValue(Number(value)),
          labels.get(String(name)) ?? String(name),
        ]}
        {...TOOLTIP_PROPS}
      />
    </>
  );

  return (
    <div className="rounded-lg border border-border bg-surface p-6">
      <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-text-secondary">
        {title}
      </h3>
      <div className="h-64 w-full">
        <ResponsiveContainer width="100%" height="100%">
          {variant === 'area' ? (
            <AreaChart data={data} margin={CHART_MARGIN}>
              <defs>
                {series.map((item) => (
                  <linearGradient
                    key={item.dataKey}
                    id={`${gradientId}-${item.dataKey}`}
                    x1="0"
                    y1="0"
                    x2="0"
                    y2="1"
                  >
                    <stop
                      offset="5%"
                      stopColor={item.color}
                      stopOpacity={0.2}
                    />
                    <stop offset="95%" stopColor={item.color} stopOpacity={0} />
                  </linearGradient>
                ))}
              </defs>
              {axes}
              {series.map((item) => (
                <Area
                  key={item.dataKey}
                  type="monotone"
                  dataKey={item.dataKey}
                  name={item.dataKey}
                  stroke={item.color}
                  strokeWidth={1.5}
                  fillOpacity={1}
                  fill={`url(#${gradientId}-${item.dataKey})`}
                  isAnimationActive={false}
                />
              ))}
            </AreaChart>
          ) : (
            <LineChart data={data} margin={CHART_MARGIN}>
              {axes}
              {series.map((item) => (
                <Line
                  key={item.dataKey}
                  type="monotone"
                  dataKey={item.dataKey}
                  name={item.dataKey}
                  stroke={item.color}
                  strokeWidth={1.5}
                  dot={false}
                  activeDot={{ r: 4 }}
                  isAnimationActive={false}
                />
              ))}
            </LineChart>
          )}
        </ResponsiveContainer>
      </div>
    </div>
  );
}
