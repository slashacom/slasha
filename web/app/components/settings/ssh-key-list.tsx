import { KeyIcon } from 'lucide-react';
import type { SshKey } from '~/models/ssh-key';
import { SshKeyRow } from './ssh-key-row';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';

type SshKeyListProps = {
  keys: SshKey[];
  isLoading: boolean;
  onDelete: (key: SshKey) => void;
  onAddFirst: () => void;
};

export function SshKeyList(props: SshKeyListProps) {
  const { keys, isLoading, onDelete, onAddFirst } = props;
  return (
    <div className="mt-2 min-w-0 flex-1 overflow-x-auto">
      {isLoading ? (
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => (
            <div
              key={i}
              className="h-10 w-full animate-pulse rounded border border-border bg-surface/50"
            />
          ))}
        </div>
      ) : keys.length === 0 ? (
        <EmptyPage
          dashed
          icon={KeyIcon}
          title="No SSH keys found"
          subtitle="Add a public key to access your applications via Git over SSH."
          actionLabel="Add your first key"
          onAction={onAddFirst}
        />
      ) : (
        <Table
          columns={[
            'Title',
            'Public Key',
            'Created',
            { label: '', align: 'right' },
          ]}
        >
          {keys.map((key) => (
            <SshKeyRow key={key.id} sshKey={key} onDelete={onDelete} />
          ))}
        </Table>
      )}
    </div>
  );
}
