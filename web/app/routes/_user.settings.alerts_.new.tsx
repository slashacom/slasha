import { ArrowLeft } from 'lucide-react';
import { useNavigate } from 'react-router';
import { RuleForm } from '~/components/alerting/rule-form';
import { HStack } from '~/components/interface/stacks';
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
  const navigate = useNavigate();

  return (
    <div className="max-w-xl space-y-6">
      <div>
        <HStack space={3}>
          <button
            type="button"
            onClick={() => navigate('/settings/alerts')}
            className="group flex size-7 shrink-0 items-center justify-center rounded border border-border bg-surface transition-all hover:bg-white/[0.06]"
          >
            <ArrowLeft className="size-3.5 text-text-tertiary group-hover:text-text" />
          </button>
          <h3 className="font-semibold text-text">New alert rule</h3>
        </HStack>
        <p className="mt-2 text-sm text-text-secondary">
          When the condition is met, perform the action.
        </p>
      </div>

      <RuleForm />
    </div>
  );
}
