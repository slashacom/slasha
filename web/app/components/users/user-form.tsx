import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack, HStack } from '~/components/interface/stacks';
import type { User } from '~/models/user';

interface UserFormProps {
  initialData?: User;
  onSubmit: (e: React.SubmitEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  isPending: boolean;
  showPassword?: boolean;
  submitLabel: string;
}

export function UserForm({
  initialData,
  onSubmit,
  onCancel,
  isPending,
  showPassword = false,
  submitLabel,
}: UserFormProps) {
  return (
    <form onSubmit={onSubmit} className="w-full">
      <VStack space={6}>
        <VStack space={2}>
          <Label htmlFor="email">Email Address</Label>
          <Input
            id="email"
            name="email"
            type="username"
            required
            defaultValue={initialData?.email}
            placeholder="user@example.com"
            className="h-11"
          />
        </VStack>

        {showPassword && (
          <VStack space={2}>
            <Label htmlFor="password">Password</Label>
            <Input
              id="password"
              name="password"
              type="password"
              required
              pattern=".{8,}"
              title="8 characters minimum"
              placeholder="••••••••"
              className="h-11"
            />
          </VStack>
        )}

        <VStack space={2}>
          <Label htmlFor="role">Role</Label>
          <select
            id="role"
            name="role"
            required
            defaultValue={initialData?.role || 'user'}
            className="flex h-11 w-full rounded-md border border-neutral-200 bg-white px-3 py-2 text-sm ring-offset-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-neutral-950 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <option value="user">User</option>
            <option value="admin">Admin</option>
          </select>
        </VStack>

        <HStack space={3} justifyContent="end" className="pt-2">
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
        </HStack>
      </VStack>
    </form>
  );
}
