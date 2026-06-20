import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import type { User } from '~/models/user';

type UserFormProps = {
  initialData?: User;
  onSubmit: (e: React.SubmitEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  isPending: boolean;
  showPassword?: boolean;
  submitLabel: string;
};

export function UserForm(props: UserFormProps) {
  const {
    initialData,
    onSubmit,
    onCancel,
    isPending,
    showPassword = false,
    submitLabel,
  } = props;
  return (
    <form onSubmit={onSubmit} className="w-full max-w-md">
      <div className="space-y-5">
        <div className="space-y-1.5">
          <Label
            htmlFor="email"
            className="text-[13px] font-medium text-text-secondary"
          >
            Email address
          </Label>
          <Input
            id="email"
            name="email"
            type="email"
            required
            defaultValue={initialData?.email}
            placeholder="user@example.com"
            className="h-10"
          />
        </div>

        {showPassword && (
          <div className="space-y-1.5">
            <Label
              htmlFor="password"
              className="text-[13px] font-medium text-text-secondary"
            >
              Password
            </Label>
            <Input
              id="password"
              name="password"
              type="password"
              required
              pattern=".{8,}"
              title="8 characters minimum"
              placeholder="At least 8 characters"
              className="h-10"
            />
          </div>
        )}

        <div className="space-y-1.5">
          <Label
            htmlFor="role"
            className="text-[13px] font-medium text-text-secondary"
          >
            Role
          </Label>
          <select
            id="role"
            name="role"
            required
            defaultValue={initialData?.role || 'user'}
            className="flex h-10 w-full rounded-md border border-border bg-surface px-3 py-2 text-sm text-text outline-none transition-colors focus:border-text-secondary disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="user">User</option>
            <option value="admin">Admin</option>
          </select>
        </div>

        <div className="flex items-center justify-end gap-2 pt-2">
          <Button
            variant="ghost"
            label="Cancel"
            onClick={onCancel}
            isDisabled={isPending}
          />
          <Button
            type="submit"
            label={submitLabel}
            isLoading={isPending}
            isDisabled={isPending}
          />
        </div>
      </div>
    </form>
  );
}
