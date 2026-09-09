import { type ComponentPropsWithRef, useState } from 'react';
import { Eye, EyeOff } from 'lucide-react';
import { Input } from '~/components/interface/input';
import { cn } from '~/utils/classname';

export function PasswordInput(props: ComponentPropsWithRef<'input'>) {
  const { className, ...rest } = props;
  const [isVisible, setIsVisible] = useState(false);
  const Icon = isVisible ? EyeOff : Eye;

  return (
    <div className="relative">
      <Input
        {...rest}
        type={isVisible ? 'text' : 'password'}
        className={cn('pr-9', className)}
      />
      <button
        type="button"
        tabIndex={-1}
        aria-label={isVisible ? 'Hide password' : 'Show password'}
        onClick={() => setIsVisible(!isVisible)}
        className="absolute inset-y-0 right-0 flex w-9 cursor-pointer items-center justify-center text-text-tertiary transition-colors hover:text-text"
      >
        <Icon className="size-4" />
      </button>
    </div>
  );
}
