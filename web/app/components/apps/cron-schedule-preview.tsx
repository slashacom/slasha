import { describeSchedule } from '~/utils/cron';
import { formatDateTime } from '~/utils/date';

type CronSchedulePreviewProps = {
  schedule: string;
  loading: boolean;
  error: string | null;
  nextRuns: string[];
};

export function CronSchedulePreview(props: CronSchedulePreviewProps) {
  const { schedule, loading, error, nextRuns } = props;

  if (error) {
    return <p className="text-[11px] text-red-400">{error}</p>;
  }

  if (loading && nextRuns.length === 0) {
    return (
      <p className="text-[11px] text-text-tertiary">Checking schedule...</p>
    );
  }

  if (nextRuns.length === 0) {
    return null;
  }

  const description = describeSchedule(schedule);

  return (
    <div className="rounded-md border border-border bg-bg/40 px-3 py-2">
      {description ? <p className="text-xs text-text">{description}</p> : null}
      <p className="mt-1 text-[11px] text-text-tertiary">Next runs</p>
      <div className="mt-1 space-y-0.5">
        {nextRuns.map((run) => (
          <div key={run} className="text-[11px] text-text-secondary">
            {formatDateTime(run)}
          </div>
        ))}
      </div>
    </div>
  );
}
