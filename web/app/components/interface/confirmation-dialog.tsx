import type { ReactNode } from 'react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from './alert-dialog';

import { AlertTriangle } from 'lucide-react';

interface ConfirmationDialogProps {
  icon: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  onConfirm?: () => void;
}

export function ConfirmationDialog(props: ConfirmationDialogProps) {
  const {
    icon = <AlertTriangle className="h-7 w-7 text-neutral-500" />,
    open,
    onOpenChange,
    title,
    description,
    onConfirm,
  } = props;

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="mx-auto w-[320px] !rounded-xl border-neutral-200 bg-white text-center">
        <AlertDialogHeader className="flex flex-col items-center">
          <div className="mt-2 mb-5">{icon}</div>
          <AlertDialogTitle className="text-center text-lg font-medium text-neutral-900">
            {title}
          </AlertDialogTitle>
          <AlertDialogDescription className="text-center text-sm text-balance text-neutral-600">
            {description}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="flex gap-2">
          <AlertDialogCancel
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onOpenChange(false);
            }}
            className="flex-grow text-sm cursor-pointer rounded-md bg-neutral-200/70 text-neutral-700 outline-neutral-300 hover:bg-neutral-300/70 focus:outline-2 border-0"
          >
            Cancel
          </AlertDialogCancel>
          <AlertDialogAction
            className="flex-grow text-sm cursor-pointer rounded-md bg-red-600 px-4 py-2 text-white outline-red-400 hover:bg-red-500 focus:outline-2"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onConfirm?.();
              onOpenChange(false);
            }}
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
