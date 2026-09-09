export const CHART_COLORS = {
  cpu: '#06b6d4',
  memory: '#a855f7',
  networkRx: '#3b82f6',
  networkTx: '#f97316',
  diskRead: '#f43f5e',
  diskWrite: '#f59e0b',
  load: '#22c55e',
} as const;

export const CHART_MARGIN = { top: 10, right: 5, left: -20, bottom: 0 };

export const GRID_STROKE = 'rgba(255, 255, 255, 0.05)';

export const AXIS_PROPS = {
  stroke: 'rgba(255, 255, 255, 0.3)',
  fontSize: 10,
  tickLine: false,
} as const;

export const TOOLTIP_PROPS = {
  contentStyle: {
    backgroundColor: '#232323',
    borderColor: 'rgba(255, 255, 255, 0.08)',
    borderRadius: '8px',
    color: '#fff',
  },
  labelStyle: {
    color: 'rgba(255, 255, 255, 0.5)',
    fontSize: '11px',
  },
  itemStyle: { fontSize: '12px' },
} as const;
