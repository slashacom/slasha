import * as React from 'react';
import { Checkbox as CheckboxPrimitive } from 'radix-ui';
import { CheckIcon } from 'lucide-react';

import { cn } from '~/utils/classname';

function Checkbox(
  componentProps: React.ComponentProps<typeof CheckboxPrimitive.Root>
) {
  const { className, ...props } = componentProps;
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      className={cn(
        'peer border-border hover:border-text-secondary data-[state=checked]:bg-text data-[state=checked]:text-bg data-[state=checked]:border-text focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 aria-invalid:border-destructive size-4 shrink-0 rounded-[4px] border outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50',
        className
      )}
      {...props}
    >
      <CheckboxPrimitive.Indicator
        data-slot="checkbox-indicator"
        className="flex items-center justify-center text-current transition-none"
      >
        <CheckIcon className="size-3.5" />
      </CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  );
}

export { Checkbox };
