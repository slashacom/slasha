import { useNavigate, useParams } from 'react-router';
import { ArrowLeft, Clock } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { CronForm } from '~/components/apps/cron-form';

export default function NewCronPage() {
  const { slug } = useParams();
  const navigate = useNavigate();

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <SectionHeader
        icon={Clock}
        title="New cron job"
        actions={
          <Button
            to={`/apps/${slug}/crons`}
            label="Back"
            variant="ghost"
            icon={<ArrowLeft className="size-4" />}
          />
        }
      />

      <div className="p-8">
        <CronForm
          appSlug={slug!}
          onCancel={() => navigate(`/apps/${slug}/crons`)}
          onSaved={() => navigate(`/apps/${slug}/crons`)}
        />
      </div>
    </div>
  );
}
