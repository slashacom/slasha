import { KeyIcon, Trash2Icon } from 'lucide-react';
import type { SshKey } from '~/models/ssh-key';
import { formatDate } from '~/utils/date';
import { TableRowActions } from '~/components/interface/table-row-actions';

type SshKeyRowProps = {
  sshKey: SshKey;
  onDelete: (key: SshKey) => void;
};

export function SshKeyRow(props: SshKeyRowProps) {
  const { sshKey, onDelete } = props;
  return (
    <tr className="group transition-colors hover:bg-white/[0.01]">
      <td className="py-4 pr-4 align-top">
        <div className="flex items-center gap-2.5">
          <KeyIcon className="size-3.5 text-text-tertiary" />
          <span className="font-medium text-text">{sshKey.name}</span>
        </div>
      </td>
      <td className="py-4 pr-4 align-top">
        <code className="block max-w-[320px] truncate rounded bg-surface/50 px-1.5 py-0.5 font-mono text-[11px] text-text-tertiary transition-all group-hover:max-w-md">
          {sshKey.public_key}
        </code>
      </td>
      <td className="py-4 pr-4 align-top text-text-tertiary">
        {formatDate(sshKey.created_at)}
      </td>
      <td className="py-4 text-right align-top">
        <TableRowActions
          actions={[
            {
              label: 'Delete key',
              icon: Trash2Icon,
              isDestructive: true,
              onClick: () => onDelete(sshKey),
            },
          ]}
        />
      </td>
    </tr>
  );
}
