import { useNavigate } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { AlertRuleForm } from '~/components/alerts/alert-rule-form';
import { getAppsOptions } from '~/queries/apps';
import { getAlertChannelsOptions, getAllCronsOptions } from '~/queries/alerts';
import { getNodesOptions } from '~/queries/nodes';
import { queryClient } from '~/utils/query-client';
import { PageHeader } from '~/components/interface/page-header';

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
    <div className="px-8 py-6">
      <PageHeader
        className="mb-8"
        title="New rule"
        description="Define a condition and choose how notifications should be delivered."
      />
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
