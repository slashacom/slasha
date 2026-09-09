import { FormActions } from '~/components/interface/form-actions';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Textarea } from '~/components/interface/textarea';

type SshKeyFormProps = {
  onSubmit: (e: React.SubmitEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  isPending: boolean;
  submitLabel: string;
};

export function SshKeyForm(props: SshKeyFormProps) {
  const { onSubmit, onCancel, isPending, submitLabel } = props;

  return (
    <form onSubmit={onSubmit} className="w-full max-w-xl">
      <div className="space-y-5">
        <div className="space-y-1.5">
          <Label
            htmlFor="name"
            className="text-[13px] font-medium text-text-secondary"
          >
            Name
          </Label>
          <Input
            id="name"
            name="name"
            required
            autoFocus
            placeholder="e.g. My Laptop"
            className="h-10"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="public_key"
            className="text-[13px] font-medium text-text-secondary"
          >
            Public key
          </Label>
          <Textarea
            id="public_key"
            name="public_key"
            required
            placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA..."
            className="min-h-[140px] font-mono text-xs text-text"
          />
        </div>

        <FormActions
          submitLabel={submitLabel}
          onCancel={onCancel}
          isPending={isPending}
        />
      </div>
    </form>
  );
}
