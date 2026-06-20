import * as AlertDialogPrimitive from '@radix-ui/react-alert-dialog';

type ConfirmationDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel?: string;
  onConfirm: () => void;
}

export function ConfirmationDialog(props: ConfirmationDialogProps) {
  const {
    open,
    onOpenChange,
    title,
    description,
    confirmLabel = 'Confirm',
    onConfirm,
  } = props;

  return (
    <AlertDialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialogPrimitive.Portal>
        <AlertDialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <AlertDialogPrimitive.Content className="fixed top-1/2 left-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-border bg-surface p-6 shadow-2xl data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95">
          <AlertDialogPrimitive.Title className="text-[14px] font-semibold text-text">
            {title}
          </AlertDialogPrimitive.Title>
          <AlertDialogPrimitive.Description className="mt-2 text-[13px] leading-relaxed text-text-secondary">
            {description}
          </AlertDialogPrimitive.Description>
          <div className="mt-5 flex justify-end gap-2">
            <AlertDialogPrimitive.Cancel className="cursor-pointer rounded-md px-3 py-1.5 text-[13px] text-text-secondary outline-none transition-colors hover:bg-white/5 hover:text-text">
              Cancel
            </AlertDialogPrimitive.Cancel>
            <AlertDialogPrimitive.Action
              onClick={onConfirm}
              className="cursor-pointer rounded-md bg-red-600 px-3 py-1.5 text-[13px] font-medium text-white outline-none transition-colors hover:bg-red-500"
            >
              {confirmLabel}
            </AlertDialogPrimitive.Action>
          </div>
        </AlertDialogPrimitive.Content>
      </AlertDialogPrimitive.Portal>
    </AlertDialogPrimitive.Root>
  );
}
