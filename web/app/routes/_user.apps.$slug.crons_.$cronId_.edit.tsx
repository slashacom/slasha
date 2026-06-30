import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, Clock } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
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
      <div className="p-8 text-sm text-text-secondary">Cron job not found.</div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <SectionHeader
        icon={Clock}
        title={`Edit ${cron.name}`}
        actions={
          <Button
            to={`/apps/${slug}/crons/${cron.id}`}
            label="Back"
            variant="ghost"
            icon={<ArrowLeft className="size-4" />}
          />
        }
      />

      <div className="p-8">
        <CronForm
          appSlug={slug!}
          cron={cron}
          onCancel={() => navigate(`/apps/${slug}/crons/${cron.id}`)}
          onSaved={() => navigate(`/apps/${slug}/crons/${cron.id}`)}
        />
      </div>
    </div>
  );
}
