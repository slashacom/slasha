import type { ComponentPropsWithRef } from 'react';
import { cn } from '~/utils/classname';

export function Input(props: ComponentPropsWithRef<'input'>) {
  const { className, ...rest } = props;
  return (
    <input
      type="text"
      {...rest}
      className={cn(
        'flex h-9 w-full rounded-md border border-gray-300 bg-white px-2 focus-visible:border-gray-400 py-2 text-sm outline-hidden file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-gray-500 focus-visible:outline-none disabled:cursor-not-allowed disabled:bg-gray-50 disabled:opacity-70',
        className
      )}
    />
  );
}
