import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import { CronForm } from '~/components/apps/cron-form';
import { getCronsOptions } from '~/queries/crons';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getCronsOptions(params.slug));
}

export default function EditCronPage() {
  const { slug, cronId } = useParams();
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getCronsOptions(slug!));
  const cron = data.crons.find((item) => item.id === cronId);

  if (!cron) {
    return (
      <Page>
        <p className="text-sm text-text-secondary">Cron job not found.</p>
      </Page>
    );
  }

  return (
    <Page className="min-h-0 flex-1 overflow-y-auto">
      <PageHeader title={`Edit ${cron.name}`} />

      <div className="mt-6">
        <CronForm
          appSlug={slug!}
          cron={cron}
          onCancel={() => navigate(`/apps/${slug}/crons/${cron.id}`)}
          onSaved={() => navigate(`/apps/${slug}/crons/${cron.id}`)}
        />
      </div>
    </Page>
  );
}
