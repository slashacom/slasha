import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertRuleForm } from '~/components/alerts/alert-rule-form';
import { getAppsOptions } from '~/queries/apps';
import {
  getAlertChannelsOptions,
  getAlertRulesOptions,
  getAllCronsOptions,
} from '~/queries/alerts';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getAlertChannelsOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
    queryClient.ensureQueryData(getAllCronsOptions()),
  ]);
  return null;
}

export default function EditAlertRulePage() {
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const { data: rulesData } = useSuspenseQuery(getAlertRulesOptions());
  const { data: channelsData } = useSuspenseQuery(getAlertChannelsOptions());
  const { data: appsData } = useSuspenseQuery(getAppsOptions());
  const { data: cronsData } = useSuspenseQuery(getAllCronsOptions());
  const rule = rulesData.rules.find((item) => item.id === id);

  if (!rule) {
    return (
      <div className="p-8 text-sm text-text-secondary">
        Alert rule not found.
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8">
        <h2 className="text-base font-semibold text-text">Edit rule</h2>
        <p className="mt-1 text-sm text-text-tertiary">
          Update {rule.name} and its delivery behavior.
        </p>
      </div>
      <AlertRuleForm
        rule={rule}
        apps={appsData.apps.map((item) => item.app)}
        channels={channelsData.channels}
        crons={cronsData.crons}
        onCancel={() => navigate('/alerts/rules')}
        onSaved={() => navigate('/alerts/rules')}
      />
    </div>
  );
}
