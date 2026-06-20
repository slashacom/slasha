import { toast } from 'sonner';
import { Button } from '../interface/button';
import { Input } from '../interface/input';
import { Label } from '../interface/label';
import { Textarea } from '../interface/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../interface/dialog';
import { useCreateSshKey } from '~/queries/ssh-keys';

type AddSshKeyDialogProps = {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AddSshKeyDialog({
  isOpen,
  onOpenChange,
}: AddSshKeyDialogProps) {
  const createKey = useCreateSshKey();

  const handleAddKey = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const title = formData.get('title') as string;
    const public_key = formData.get('public_key') as string;

    const promise = createKey.mutateAsync({
      title: title || undefined,
      public_key,
    });

    toast.promise(promise, {
      loading: 'Adding SSH key...',
      success: 'SSH key added successfully',
      error: (err) => err.message || 'Failed to add SSH key.',
    });

    try {
      await promise;
      onOpenChange(false);
    } catch {}
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Add SSH Key</DialogTitle>
          <DialogDescription>
            Provide a title to identify this key and paste your public key
            below.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleAddKey}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="title">Title</Label>
              <Input
                id="title"
                name="title"
                placeholder="e.g. My Laptop"
                autoFocus
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="public_key">Public Key</Label>
              <Textarea
                id="public_key"
                name="public_key"
                required
                placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA..."
                className="min-h-[120px] font-mono text-xs text-text"
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              label="Cancel"
              onClick={() => onOpenChange(false)}
            />
            <Button
              type="submit"
              label="Add key"
              isLoading={createKey.isPending}
              isDisabled={createKey.isPending}
            />
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
