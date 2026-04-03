import * as React from 'react';
import { cn } from '~/utils/classname';

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<'input'>>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          'block h-9 w-full rounded-lg border border-gray-200 bg-white px-3 py-2 outline-none placeholder:text-gray-500 focus:border-gray-500 disabled:bg-gray-50',
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';

export { Input };
