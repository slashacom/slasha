import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { Page } from '~/components/global/page';
import { Button } from '~/components/interface/button';
import { AppList } from '~/components/apps/app-list';
import { getAppsOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';
import { PageHeader } from '~/components/interface/page-header';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAppsOptions());
}

export default function AppsIndex() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getAppsOptions());

  return (
    <Page>
      <PageHeader
        title="Apps"
        description="Manage and browse the applications running on this instance."
        actions={
          <Button
            label="New app"
            icon={<PlusIcon className="size-4" />}
            onClick={() => navigate('/apps/new')}
          />
        }
      />

      <div className="mt-6">
        <AppList apps={data.apps ?? []} />
      </div>
    </Page>
  );
}
