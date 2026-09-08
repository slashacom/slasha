import type { AlertIncident } from '~/models/alerts';
import { formatMetric } from '~/utils/format';

export type IncidentValue = {
  label: string;
  value: string;
};

const VALUE_LABELS = [
  ['trigger_value', 'Trigger value'],
  ['current_value', 'Current value'],
  ['recovery_value', 'Recovery value'],
  ['threshold_value', 'Threshold value'],
] as const;

export function incidentValues(incident: AlertIncident): IncidentValue[] {
  return VALUE_LABELS.filter(([key]) => incident[key] !== null).map(
    ([key, label]) => ({ label, value: formatMetric(incident[key]) })
  );
}

export function incidentValueSummary(incident: AlertIncident): string | null {
  const values = incidentValues(incident);
  if (values.length === 0) {
    return null;
  }

  return values
    .map((entry) => `${entry.label.replace(' value', '')} ${entry.value}`)
    .join(' · ');
}
