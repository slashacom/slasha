import { useParams, useSearchParams } from 'react-router';
import { Terminal } from 'lucide-react';
import { HStack } from '~/components/interface/stacks';
import { LogStream } from '~/components/global/log-stream';
import { SectionHeader } from '~/components/interface/section-header';

export default function NodeLogsTab() {
  const { id } = useParams<{ id: string }>();
  const [searchParams, setSearchParams] = useSearchParams();

  const logType =
    searchParams.get('type') === 'teardown' ? 'teardown' : 'setup';

  const handleToggleLogType = (type: 'setup' | 'teardown') => {
    setSearchParams({ type });
  };

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <SectionHeader
        icon={Terminal}
        title="Logs"
        actions={
          <HStack
            space={1}
            className="rounded border border-border bg-surface p-0.5"
          >
            <button
              type="button"
              onClick={() => handleToggleLogType('setup')}
              className={`h-7 px-3 rounded text-[11px] font-medium transition-colors ${
                logType === 'setup'
                  ? 'bg-white/[0.08] text-text'
                  : 'text-text-tertiary hover:text-text'
              }`}
            >
              Setup logs
            </button>
            <button
              type="button"
              onClick={() => handleToggleLogType('teardown')}
              className={`h-7 px-3 rounded text-[11px] font-medium transition-colors ${
                logType === 'teardown'
                  ? 'bg-white/[0.08] text-text'
                  : 'text-text-tertiary hover:text-text'
              }`}
            >
              Teardown logs
            </button>
          </HStack>
        }
      />
      <div className="flex-1 overflow-hidden p-8 flex flex-col min-h-0">
        <LogStream
          url={`/api/nodes/${id}/logs?type=${logType}`}
          emptyMessage={
            logType === 'setup'
              ? 'No setup logs found.'
              : 'No teardown logs found.'
          }
          className="min-h-0 flex-1 rounded-lg border border-border"
        />
      </div>
    </div>
  );
}
