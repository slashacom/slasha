import { useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { RuleForm } from '~/components/alerting/rule-form';
import { getAlertRulesOptions, getChannelsOptions } from '~/queries/alerting';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Edit rule' }];
}

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getAlertRulesOptions()),
    queryClient.ensureQueryData(getChannelsOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function EditRulePage() {
  const { ruleId } = useParams();
  const { data: rules } = useSuspenseQuery(getAlertRulesOptions());
  const rule = rules.find((entry) => entry.id === ruleId);

  return (
    <div className="max-w-xl space-y-6">
      <div>
        <h3 className="font-semibold text-text">Edit alert rule</h3>
        <p className="mt-2 text-sm text-text-secondary">
          When the condition is met, perform the action.
        </p>
      </div>

      {rule ? (
        <RuleForm rule={rule} />
      ) : (
        <p className="text-sm text-text-tertiary">
          This rule no longer exists.
        </p>
      )}
    </div>
  );
}
