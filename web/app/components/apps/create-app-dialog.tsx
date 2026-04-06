import * as React from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '~/components/interface/dialog';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import { useCreateApp } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

interface CreateAppDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CreateAppDialog({ open, onOpenChange }: CreateAppDialogProps) {
  const navigate = useNavigate();
  const createApp = useCreateApp();

  const handleSubmit = async (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get('name') as string;

    const promise = createApp.mutateAsync({ name });

    toast.promise(promise, {
      loading: 'Creating application...',
      success: (data) => {
        queryClient.invalidateQueries({ queryKey: ['apps'] });
        onOpenChange(false);
        navigate(`/apps/${data.app.slug}`);
        return `Successfully created ${name}`;
      },
      error: (err) => err.message || 'Failed to create application.',
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Create a New App</DialogTitle>
          <DialogDescription>
            Give your application a name to get started. We'll set up a git
            repository for you.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <VStack space={6} className="py-4">
            <VStack space={2}>
              <Label htmlFor="name" className="text-sm font-medium">
                Application Name
              </Label>
              <Input
                id="name"
                name="name"
                type="text"
                required
                placeholder="my-awesome-app"
                autoFocus
                className="h-10 border-neutral-200 focus-visible:border-black focus-visible:ring-0 transition-all"
              />
              <p className="text-xs text-neutral-400">
                This will be used to generate your application slug and
                repository name.
              </p>
            </VStack>
            <DialogFooter>
              <Button
                type="button"
                variant="ghost"
                label="Cancel"
                onClick={() => onOpenChange(false)}
                isDisabled={createApp.isPending}
              />
              <Button
                type="submit"
                label="Create App"
                isLoading={createApp.isPending}
                isDisabled={createApp.isPending}
                className="shadow-sm"
              />
            </DialogFooter>
          </VStack>
        </form>
      </DialogContent>
    </Dialog>
  );
}
