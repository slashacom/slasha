import type { ComponentPropsWithRef } from 'react';
import { cn } from '~/utils/classname';

export function Input(props: ComponentPropsWithRef<'input'>) {
  const { className, ...rest } = props;
  return (
    <input
      type="text"
      {...rest}
      className={cn(
        'flex h-9 w-full rounded-md border border-border bg-surface px-3 py-2 text-sm text-text outline-none transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-text-tertiary focus:border-text-secondary disabled:cursor-not-allowed disabled:opacity-60',
        className
      )}
    />
  );
}
