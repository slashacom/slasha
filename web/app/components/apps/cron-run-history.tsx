import { useState } from 'react';
import { History } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { HStack } from '~/components/interface/stacks';
import { Table } from '~/components/interface/table';
import { CronRunStatusBadge } from '~/components/apps/cron-run-status-badge';
import { LogStream } from '~/components/global/log-stream';
import type { CronRun } from '~/models/cron';
import {
  formatDateTime,
  formatDuration,
  formatRelativeTime,
} from '~/utils/date';

type CronRunHistoryProps = {
  appSlug: string;
  cronId: string;
  runs: CronRun[];
};

function runStartedAt(run: CronRun) {
  return run.started_at ?? run.created_at;
}

function runDuration(run: CronRun) {
  if (!run.started_at) {
    return '—';
  }
  if (run.status === 'running') {
    return formatDuration(run.started_at);
  }
  if (!run.finished_at) {
    return '—';
  }
  return formatDuration(run.started_at, run.finished_at);
}

type RunOutcomeProps = {
  run: CronRun;
};

function RunOutcome(props: RunOutcomeProps) {
  const { run } = props;

  if (run.error) {
    return (
      <span
        className="block max-w-[420px] truncate text-red-300/90"
        title={run.error}
      >
        {run.error}
      </span>
    );
  }

  if (run.exit_code !== null) {
    return <span className="font-mono text-xs">exit {run.exit_code}</span>;
  }

  return <span className="text-text-tertiary">—</span>;
}

export function CronRunHistory(props: CronRunHistoryProps) {
  const { appSlug, cronId, runs } = props;
  const [selectedRun, setSelectedRun] = useState<CronRun | null>(null);

  if (runs.length === 0) {
    return (
      <EmptyPage
        icon={History}
        size="sm"
        bordered={false}
        title="No runs yet."
        subtitle="Runs appear here once the schedule fires or you trigger the job manually."
      />
    );
  }

  return (
    <>
      <div className="overflow-x-auto">
        <Table
          columns={[
            'Status',
            'Started',
            'Duration',
            'Trigger',
            'Result',
            { label: '', align: 'right' },
          ]}
        >
          {runs.map((run) => (
            <tr
              key={run.id}
              onClick={() => setSelectedRun(run)}
              className="group cursor-pointer transition-colors hover:bg-white/[0.02]"
            >
              <td className="py-3 pr-4">
                <CronRunStatusBadge status={run.status} />
              </td>
              <td className="py-3 pr-4">
                <div className="text-text">
                  {formatRelativeTime(runStartedAt(run))}
                </div>
                <div className="mt-0.5 text-xs text-text-tertiary">
                  {formatDateTime(runStartedAt(run))}
                </div>
              </td>
              <td className="py-3 pr-4 tabular-nums text-text-secondary">
                {runDuration(run)}
              </td>
              <td className="py-3 pr-4 capitalize text-text-secondary">
                {run.trigger_kind}
              </td>
              <td className="py-3 pr-4 text-text-secondary">
                <RunOutcome run={run} />
              </td>
              <td className="py-3 text-right">
                <span className="text-xs text-text-tertiary opacity-0 transition-opacity group-hover:opacity-100">
                  View logs
                </span>
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
              {selectedRun
                ? `${formatDateTime(runStartedAt(selectedRun))} · ${runDuration(selectedRun)}`
                : ''}
            </DialogDescription>
          </DialogHeader>

          {selectedRun ? (
            <HStack space={2} wrap>
              <CronRunStatusBadge status={selectedRun.status} />
              <span className="text-xs capitalize text-text-tertiary">
                {selectedRun.trigger_kind}
              </span>
              {selectedRun.exit_code !== null ? (
                <span className="font-mono text-xs text-text-tertiary">
                  exit {selectedRun.exit_code}
                </span>
              ) : null}
            </HStack>
          ) : null}

          {selectedRun?.error ? (
            <p className="max-h-32 overflow-y-auto whitespace-pre-wrap break-words rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300">
              {selectedRun.error}
            </p>
          ) : null}

          {selectedRun ? (
            <LogStream
              url={`/api/apps/${appSlug}/crons/${cronId}/runs/${selectedRun.id}`}
              resourceKind="cron"
              className="h-[55vh]"
            />
          ) : null}
        </DialogContent>
      </Dialog>
    </>
  );
}
