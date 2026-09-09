import { useParams } from 'react-router';
import { Terminal } from 'lucide-react';
import { LogStream } from '~/components/global/log-stream';

export default function NodeLogsTab() {
  const { id } = useParams<{ id: string }>();

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden px-8 py-6">
        <LogStream
          url={`/api/nodes/${id}`}
          resourceKind="node"
          title="Logs"
          emptyMessage="No logs found."
          className="min-h-0 flex-1"
        />
      </div>
    </div>
  );
}
