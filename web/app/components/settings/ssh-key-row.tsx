import { KeyIcon, Trash2Icon } from 'lucide-react';
import type { SshKey } from '~/models/ssh_key';

interface SshKeyRowProps {
  sshKey: SshKey;
  onDelete: (key: SshKey) => void;
}

export function SshKeyRow({ sshKey, onDelete }: SshKeyRowProps) {
  return (
    <tr className="group transition-colors hover:bg-white/[0.01]">
      <td className="py-4 pr-4 align-top">
        <div className="flex items-center gap-2.5">
          <KeyIcon className="size-3.5 text-text-tertiary" />
          <span className="font-medium text-text">
            {sshKey.title || 'Untitled'}
          </span>
        </div>
      </td>
      <td className="py-4 pr-4 align-top">
        <code className="block max-w-[320px] truncate rounded bg-surface/50 px-1.5 py-0.5 font-mono text-[11px] text-text-tertiary transition-all group-hover:max-w-md">
          {sshKey.public_key}
        </code>
      </td>
      <td className="py-4 pr-4 align-top text-text-tertiary">
        {new Date(sshKey.created_at).toLocaleDateString()}
      </td>
      <td className="py-4 text-right align-top">
        <button
          onClick={() => onDelete(sshKey)}
          className="text-text-tertiary transition-colors hover:text-red-500"
          title="Delete key"
        >
          <Trash2Icon className="size-4" />
        </button>
      </td>
    </tr>
  );
}
