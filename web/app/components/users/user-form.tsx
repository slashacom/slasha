import { useState } from 'react';
import { FormActions } from '~/components/interface/form-actions';
import { Input } from '~/components/interface/input';
import { PasswordInput } from '~/components/interface/password-input';
import { Label } from '~/components/interface/label';
import { Select } from '~/components/interface/select';
import type { User, UserRole } from '~/models/user';

type UserFormProps = {
  initialData?: User;
  onSubmit: (e: React.SubmitEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  isPending: boolean;
  submitLabel: string;
};

export function UserForm(props: UserFormProps) {
  const { initialData, onSubmit, onCancel, isPending, submitLabel } = props;

  const [_role, setRole] = useState<UserRole>(initialData?.role || 'User');

  return (
    <form onSubmit={onSubmit} className="w-full max-w-xl">
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

        <div className="space-y-1.5">
          <Label
            htmlFor="password"
            className="text-[13px] font-medium text-text-secondary"
          >
            {initialData ? 'New Password (optional)' : 'Password'}
          </Label>
          <PasswordInput
            id="password"
            name="password"
            required={!initialData}
            pattern=".{8,}"
            title="8 characters minimum"
            placeholder={
              initialData
                ? 'Leave blank to keep unchanged'
                : 'At least 8 characters'
            }
            className="h-10"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="role"
            className="text-[13px] font-medium text-text-secondary"
          >
            Role
          </Label>
          <Select
            id="role"
            name="role"
            required
            defaultValue={initialData?.role || 'User'}
            onChange={(e) => setRole(e.target.value as UserRole)}
          >
            <option value="User">User</option>
            <option value="Admin">Admin</option>
          </Select>
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
