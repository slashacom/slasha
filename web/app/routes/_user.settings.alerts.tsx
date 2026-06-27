import { ChannelsSection } from '~/components/alerting/channels-section';
import { RulesSection } from '~/components/alerting/rules-section';
import { getAlertRulesOptions, getChannelsOptions } from '~/queries/alerting';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Alerts' }];
}

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getChannelsOptions()),
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function AlertsSettingsPage() {
  return (
    <div className="max-w-2xl space-y-10">
      <RulesSection />
      <ChannelsSection />
    </div>
  );
}
