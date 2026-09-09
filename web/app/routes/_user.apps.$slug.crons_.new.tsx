import { useNavigate, useParams } from 'react-router';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import { CronForm } from '~/components/apps/cron-form';

export default function NewCronPage() {
  const { slug } = useParams();
  const navigate = useNavigate();

  return (
    <Page className="min-h-0 flex-1 overflow-y-auto">
      <PageHeader
        title="New cron job"
        description="Run a command against this app on a recurring schedule, like backups, digests and cleanups."
      />

      <div className="mt-6">
        <CronForm
          appSlug={slug!}
          onCancel={() => navigate(`/apps/${slug}/crons`)}
          onSaved={() => navigate(`/apps/${slug}/crons`)}
        />
      </div>
    </Page>
  );
}
