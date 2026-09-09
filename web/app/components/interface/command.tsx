import * as React from 'react';
import { Command as CommandPrimitive } from 'cmdk';
import { SearchIcon } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '~/components/interface/dialog';
import { cn } from '~/utils/classname';

function Command(
  componentProps: React.ComponentProps<typeof CommandPrimitive>
) {
  const { className, ...props } = componentProps;
  return (
    <CommandPrimitive
      data-slot="command"
      className={cn('flex w-full flex-col overflow-hidden', className)}
      {...props}
    />
  );
}

function CommandDialog(
  componentProps: React.ComponentProps<typeof Dialog> & {
    title: string;
    description: string;
    children: React.ReactNode;
  }
) {
  const { title, description, children, ...props } = componentProps;
  return (
    <Dialog {...props}>
      <DialogContent
        showCloseButton={false}
        className="top-[15%]! max-w-xl translate-y-0! gap-0 overflow-hidden p-0"
      >
        <DialogTitle className="sr-only">{title}</DialogTitle>
        <DialogDescription className="sr-only">{description}</DialogDescription>
        <Command loop>{children}</Command>
      </DialogContent>
    </Dialog>
  );
}

function CommandInput(
  componentProps: React.ComponentProps<typeof CommandPrimitive.Input>
) {
  const { className, ...props } = componentProps;
  return (
    <div className="flex h-11 shrink-0 items-center gap-2 border-b border-border px-3">
      <SearchIcon className="size-4 shrink-0 text-text-tertiary" />
      <CommandPrimitive.Input
        data-slot="command-input"
        className={cn(
          'h-full w-full bg-transparent text-[13px] text-text outline-none placeholder:text-text-tertiary disabled:cursor-not-allowed',
          className
        )}
        {...props}
      />
    </div>
  );
}

function CommandList(
  componentProps: React.ComponentProps<typeof CommandPrimitive.List>
) {
  const { className, ...props } = componentProps;
  return (
    <CommandPrimitive.List
      data-slot="command-list"
      className={cn(
        'custom-scrollbar max-h-[340px] scroll-py-1 overflow-x-hidden overflow-y-auto p-1',
        className
      )}
      {...props}
    />
  );
}

function CommandEmpty(
  componentProps: React.ComponentProps<typeof CommandPrimitive.Empty>
) {
  const { className, ...props } = componentProps;
  return (
    <CommandPrimitive.Empty
      data-slot="command-empty"
      className={cn(
        'py-8 text-center text-[13px] text-text-tertiary',
        className
      )}
      {...props}
    />
  );
}

function CommandGroup(
  componentProps: React.ComponentProps<typeof CommandPrimitive.Group>
) {
  const { className, ...props } = componentProps;
  return (
    <CommandPrimitive.Group
      data-slot="command-group"
      className={cn(
        'text-text [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:pt-3 [&_[cmdk-group-heading]]:pb-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-text-tertiary [&_[cmdk-group-heading]]:uppercase',
        className
      )}
      {...props}
    />
  );
}

function CommandItem(
  componentProps: React.ComponentProps<typeof CommandPrimitive.Item>
) {
  const { className, ...props } = componentProps;
  return (
    <CommandPrimitive.Item
      data-slot="command-item"
      className={cn(
        "relative flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-[13px] text-text-secondary outline-hidden select-none data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 data-[selected=true]:bg-white/5 data-[selected=true]:text-text [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-text-tertiary",
        className
      )}
      {...props}
    />
  );
}

function CommandShortcut(componentProps: React.ComponentProps<'span'>) {
  const { className, ...props } = componentProps;
  return (
    <span
      data-slot="command-shortcut"
      className={cn(
        'ml-auto text-[11px] tracking-widest text-text-tertiary',
        className
      )}
      {...props}
    />
  );
}

export {
  Command,
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandShortcut,
};
