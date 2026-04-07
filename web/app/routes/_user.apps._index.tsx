import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { Skeleton } from '~/components/interface/skeleton';
import { AppList } from '~/components/apps/app-list';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAppsOptions());
}

export default function AppsIndex() {
  const navigate = useNavigate();
  const { data, isLoading } = useQuery(getAppsOptions());

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
        {isLoading ? (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {[...Array(6)].map((_, i) => (
              <Skeleton
                key={i}
                className="h-28 rounded-lg border border-border bg-surface"
              />
            ))}
          </div>
        ) : (
          <AppList apps={data?.apps ?? []} />
        )}
      </div>
    </div>
  );
}
