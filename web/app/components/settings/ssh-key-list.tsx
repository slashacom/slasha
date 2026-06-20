import { KeyIcon } from 'lucide-react';
import type { SshKey } from '~/models/ssh-key';
import { SshKeyRow } from './ssh-key-row';
import { Button } from '../interface/button';

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
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-20">
          <KeyIcon className="size-10 text-text-tertiary/20" />
          <h4 className="mt-4 font-medium text-text">No SSH keys found</h4>
          <p className="mt-1 text-sm text-text-secondary">
            Add a public key to access your applications via Git over SSH.
          </p>
          <Button
            variant="link"
            label="Add your first key"
            className="mt-4"
            onClick={onAddFirst}
          />
        </div>
      ) : (
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-border text-xs font-medium text-text-tertiary">
              <th className="pb-3 pr-4 uppercase tracking-wider">Title</th>
              <th className="pb-3 pr-4 uppercase tracking-wider">Public Key</th>
              <th className="pb-3 pr-4 uppercase tracking-wider">Created</th>
              <th className="pb-3 text-right"></th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {keys.map((key) => (
              <SshKeyRow key={key.id} sshKey={key} onDelete={onDelete} />
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
