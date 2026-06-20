import type { ComponentPropsWithRef } from 'react';
import { ChevronDown } from 'lucide-react';
import { cn } from '~/utils/classname';

export function Select(props: ComponentPropsWithRef<'select'>) {
  const { className, children, ...rest } = props;
  return (
    <div className="relative">
      <select
        {...rest}
        className={cn(
          'flex h-9 w-full cursor-pointer appearance-none rounded-md border border-border bg-surface px-3 py-2 pr-9 text-sm text-text outline-none transition-colors focus:border-text-secondary disabled:cursor-not-allowed disabled:opacity-60',
          className
        )}
      >
        {children}
      </select>
      <ChevronDown className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
    </div>
  );
}
