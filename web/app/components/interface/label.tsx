import { Label as LabelPrimitive } from 'radix-ui';
import * as React from 'react';
import { cn } from '~/utils/classname';

const Label = React.forwardRef<
  React.ComponentRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    className={cn(
      'inline-block text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 aria-required:after:text-red-500 aria-required:after:content-["*"]',
      className
    )}
    ref={ref}
    {...props}
  />
));
Label.displayName = LabelPrimitive.Root.displayName;

export { Label };
