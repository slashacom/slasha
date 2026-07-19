import { useNavigate } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertRuleForm } from '~/components/alerts/alert-rule-form';
import { getAppsOptions } from '~/queries/apps';
import { getAlertChannelsOptions, getAllCronsOptions } from '~/queries/alerts';
import { getNodesOptions } from '~/queries/nodes';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAppsOptions()),
    queryClient.ensureQueryData(getAlertChannelsOptions()),
    queryClient.ensureQueryData(getAllCronsOptions()),
    queryClient.ensureQueryData(getNodesOptions()),
  ]);
  return null;
}

export default function NewAlertRulePage() {
  const navigate = useNavigate();
  const { data: appsData } = useSuspenseQuery(getAppsOptions());
  const { data: channelsData } = useSuspenseQuery(getAlertChannelsOptions());
  const { data: cronsData } = useSuspenseQuery(getAllCronsOptions());
  const { data: nodesData } = useSuspenseQuery(getNodesOptions());

  return (
    <div className="p-8">
      <div className="mb-8">
        <h2 className="text-base font-semibold text-text">New rule</h2>
        <p className="mt-1 text-sm text-text-tertiary">
          Define a condition and choose how notifications should be delivered.
        </p>
      </div>
      <AlertRuleForm
        apps={appsData.apps.map((item) => item.app)}
        channels={channelsData.channels}
        crons={cronsData.crons}
        nodes={nodesData.nodes}
        onCancel={() => navigate('/alerts/rules')}
        onSaved={() => navigate('/alerts/rules')}
      />
    </div>
  );
}
