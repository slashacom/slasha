import { Slot } from 'radix-ui';
import { cva, type VariantProps } from 'class-variance-authority';
import * as React from 'react';

import { cn } from '~/utils/classname';

const buttonVariants = cva(
  'inline-flex w-full items-center justify-center gap-2 rounded-lg border border-neutral-100 text-sm font-medium outline-none focus-visible:border-gray-400/90 disabled:cursor-not-allowed disabled:opacity-60 [&_svg]:pointer-events-none [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        default: 'bg-black hover:opacity-80 border-neutral-100 text-white',
        destructive: 'bg-red-500 text-white border-red-400 hover:bg-red-500/90',
        outline:
          'border border-gray-200 bg-white hover:bg-gray-100 hover:text-black',
        secondary:
          'bg-gray-100 border-gray-200 text-black hover:bg-gray-200 hover:text-black',
        ghost:
          'hover:bg-gray-100 border-transparent hover:text-black focus-visible:bg-gray-100 focus-visible:text-black',
        link: 'text-blue-500 underline underline-offset-4 hover:text-blue-600 hover:no-underline focus:no-underline border-none rounded-none',
      },
      size: {
        default: 'h-9 px-4 py-2',
        sm: 'h-8 rounded-md px-3 py-2 text-xs',
        lg: 'h-10 rounded-md px-8 py-2',
        icon: 'h-9 w-9',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  }
);

export interface ButtonProps
  extends
    React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : 'button';
    return (
      // @ts-expect-error - Slot is not a valid JSX element type
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = 'Button';

export { Button, buttonVariants };
