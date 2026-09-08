import { KeyIcon } from 'lucide-react';
import type { SshKey } from '~/models/ssh-key';
import { SshKeyRow } from './ssh-key-row';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';

type SshKeyListProps = {
  keys: SshKey[];
  onDelete: (key: SshKey) => void;
  onAddFirst: () => void;
};

export function SshKeyList(props: SshKeyListProps) {
  const { keys, onDelete, onAddFirst } = props;
  return (
    <div className="mt-2 min-w-0 flex-1 overflow-x-auto">
      {keys.length === 0 ? (
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
            'Name',
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
