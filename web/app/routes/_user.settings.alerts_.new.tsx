import { RuleForm } from '~/components/alerting/rule-form';
import { getChannelsOptions } from '~/queries/alerting';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'New rule' }];
}

export async function clientLoader() {
  await Promise.all([
    queryClient.ensureQueryData(getChannelsOptions()),
    queryClient.ensureQueryData(getAppsOptions()),
  ]);
  return null;
}

export default function NewRulePage() {
  return (
    <div className="max-w-xl space-y-6">
      <div>
        <h3 className="font-semibold text-text">New alert rule</h3>
        <p className="mt-2 text-sm text-text-secondary">
          When the condition is met, perform the action.
        </p>
      </div>

      <RuleForm />
    </div>
  );
}
