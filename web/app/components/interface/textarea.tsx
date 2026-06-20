import { cn } from '~/utils/classname';
import { type TextareaHTMLAttributes, forwardRef } from 'react';

type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, ...props }, ref) => {
    return (
      <textarea
        className={cn(
          'flex min-h-[80px] w-full rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2 text-sm text-neutral-100 placeholder:text-neutral-500 focus:ring-2 focus:ring-neutral-800 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 focus:ring-offset-neutral-700',
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);
