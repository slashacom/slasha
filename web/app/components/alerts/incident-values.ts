import type {
  AlertIncident,
  AlertRule,
  AlertRuleConfig,
} from '~/models/alerts';
import { formatMetric } from '~/utils/format';

export type IncidentValue = {
  label: string;
  value: string;
};

const VALUE_LABELS = [
  ['trigger_value', 'Trigger'],
  ['current_value', 'Current'],
  ['recovery_value', 'Recovery'],
  ['threshold_value', 'Threshold'],
] as const;

function unitFor(config: AlertRuleConfig | undefined) {
  if (!config) {
    return '';
  }

  switch (config.kind) {
    case 'node_cpu':
    case 'node_memory':
    case 'app_cpu':
    case 'app_memory':
      return '%';
    case 'domain_tls_expiry':
      return ' days';
    default:
      return '';
  }
}

export function incidentValues(
  incident: AlertIncident,
  rule?: AlertRule
): IncidentValue[] {
  const unit = unitFor(rule?.config);

  return VALUE_LABELS.filter(([key]) => incident[key] !== null).map(
    ([key, label]) => ({
      label,
      value: `${formatMetric(incident[key])}${unit}`,
    })
  );
}

export function incidentValueSummary(
  incident: AlertIncident,
  rule?: AlertRule
): string | null {
  const values = incidentValues(incident, rule);
  if (values.length === 0) {
    return null;
  }

  return values.map((entry) => `${entry.label} ${entry.value}`).join(' · ');
}
