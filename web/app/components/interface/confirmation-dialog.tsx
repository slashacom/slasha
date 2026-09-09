import { useEffect, useState } from 'react';
import * as AlertDialogPrimitive from '@radix-ui/react-alert-dialog';
import { Loader } from '~/components/icons/loader';
import { Input } from '~/components/interface/input';
import { cn } from '~/utils/classname';

type ConfirmationDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel?: string;
  isDestructive?: boolean;
  isPending?: boolean;
  confirmText?: string;
  onConfirm: () => void;
};

export function ConfirmationDialog(props: ConfirmationDialogProps) {
  const {
    open,
    onOpenChange,
    title,
    description,
    confirmLabel = 'Confirm',
    isDestructive = true,
    isPending = false,
    confirmText,
    onConfirm,
  } = props;
  const [typed, setTyped] = useState('');

  useEffect(() => {
    if (open) {
      return;
    }

    setTyped('');
  }, [open]);

  const isConfirmable = !confirmText || typed.trim() === confirmText;

  return (
    <AlertDialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialogPrimitive.Portal>
        <AlertDialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <AlertDialogPrimitive.Content className="fixed top-1/2 left-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-border bg-surface p-6 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95">
          <AlertDialogPrimitive.Title className="text-[14px] font-semibold text-text">
            {title}
          </AlertDialogPrimitive.Title>
          <AlertDialogPrimitive.Description className="mt-2 text-[13px] leading-relaxed text-text-secondary">
            {description}
          </AlertDialogPrimitive.Description>

          {confirmText ? (
            <div className="mt-4">
              <label
                htmlFor="confirmation-input"
                className="text-[12px] text-text-tertiary"
              >
                Type <span className="font-mono text-text">{confirmText}</span>{' '}
                to confirm
              </label>
              <Input
                id="confirmation-input"
                value={typed}
                autoComplete="off"
                className="mt-1.5"
                onChange={(event) => setTyped(event.target.value)}
              />
            </div>
          ) : null}

          <div className="mt-5 flex justify-end gap-2">
            <AlertDialogPrimitive.Cancel
              disabled={isPending}
              className="cursor-pointer rounded-md px-3 py-1.5 text-[13px] text-text-secondary outline-none transition-colors hover:bg-white/5 hover:text-text disabled:cursor-not-allowed disabled:opacity-50"
            >
              Cancel
            </AlertDialogPrimitive.Cancel>
            <button
              type="button"
              disabled={isPending || !isConfirmable}
              onClick={onConfirm}
              className={cn(
                'inline-flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 text-[13px] font-medium outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50',
                isDestructive
                  ? 'bg-red-600 text-white hover:bg-red-500'
                  : 'bg-white text-bg hover:bg-white/90'
              )}
            >
              {isPending ? <Loader className="size-3.5 animate-spin" /> : null}
              {confirmLabel}
            </button>
          </div>
        </AlertDialogPrimitive.Content>
      </AlertDialogPrimitive.Portal>
    </AlertDialogPrimitive.Root>
  );
}
