import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { AppList } from '~/components/apps/app-list';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAppsOptions());
}

export default function AppsIndex() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getAppsOptions());

  return (
    <div>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Apps</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Manage and browse the applications running on this instance.
          </p>
        </div>
        <Button
          label="New app"
          icon={<PlusIcon className="size-4" />}
          onClick={() => navigate('/apps/new')}
        />
      </div>

      <div className="mt-6">
        <AppList apps={data.apps ?? []} />
      </div>
    </div>
  );
}
