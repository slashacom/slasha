import { useState } from 'react';
import { History } from 'lucide-react';
import { Button } from '~/components/interface/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';
import { CronRunStatusBadge } from '~/components/apps/cron-run-status-badge';
import { LogStream } from '~/components/apps/log-stream';
import type { CronRun } from '~/models/cron';
import { formatDate } from '~/utils/format';

type CronRunHistoryProps = {
  appSlug: string;
  cronId: string;
  runs: CronRun[];
};

function formatDuration(run: CronRun): string {
  if (!run.started_at || !run.finished_at) {
    return '—';
  }

  const ms =
    new Date(run.finished_at).getTime() - new Date(run.started_at).getTime();
  if (ms < 1000) {
    return '<1s';
  }

  const seconds = Math.round(ms / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }

  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

export function CronRunHistory(props: CronRunHistoryProps) {
  const { appSlug, cronId, runs } = props;
  const [selectedRun, setSelectedRun] = useState<CronRun | null>(null);

  if (runs.length === 0) {
    return (
      <EmptyPage
        icon={History}
        title="No runs yet."
        subtitle="Runs appear here once the job fires or you trigger it manually."
        className="min-h-[240px]"
      />
    );
  }

  return (
    <>
      <div className="overflow-x-auto">
        <Table
          columns={[
            'Status',
            'Trigger',
            'Started',
            'Duration',
            'Exit',
            { label: '', align: 'right' },
          ]}
        >
          {runs.map((run) => (
            <tr key={run.id}>
              <td className="py-3 pr-4">
                <CronRunStatusBadge status={run.status} />
              </td>
              <td className="py-3 pr-4 capitalize text-text-secondary">
                {run.trigger_kind}
              </td>
              <td className="py-3 pr-4 text-text-secondary">
                {formatDate(run.started_at)}
              </td>
              <td className="py-3 pr-4 text-text-secondary">
                {formatDuration(run)}
              </td>
              <td className="py-3 pr-4 text-text-secondary">
                {run.exit_code ?? '—'}
              </td>
              <td className="py-3 text-right">
                <Button
                  label="Logs"
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelectedRun(run)}
                />
              </td>
            </tr>
          ))}
        </Table>
      </div>

      <Dialog
        open={selectedRun !== null}
        onOpenChange={(open) => !open && setSelectedRun(null)}
      >
        <DialogContent className="max-h-[85vh] overflow-hidden sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>Run logs</DialogTitle>
            <DialogDescription>
              {selectedRun ? formatDate(selectedRun.started_at) : ''}
            </DialogDescription>
          </DialogHeader>

          {selectedRun?.error ? (
            <p className="rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300">
              {selectedRun.error}
            </p>
          ) : null}

          {selectedRun ? (
            <LogStream
              url={`/api/apps/${appSlug}/crons/${cronId}/runs/${selectedRun.id}/logs`}
              className="h-[55vh] rounded-md border border-border"
            />
          ) : null}
        </DialogContent>
      </Dialog>
    </>
  );
}
