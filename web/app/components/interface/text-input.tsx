import type { ComponentProps } from 'react';
import { Check, Loader2, X } from 'lucide-react';
import { cn } from '~/utils/classname';

type TextInputProps = Omit<ComponentProps<'input'>, 'onChange'> & {
  value: string;
  onChange: (value: string) => void;
  isLoading?: boolean;
  isValid?: boolean;
  isInvalid?: boolean;
  prefix?: string;
  wrapperClassName?: string;
};

export function TextInput(props: TextInputProps) {
  const {
    value,
    onChange,
    className = '',
    isLoading,
    isValid,
    isInvalid,
    prefix,
    wrapperClassName,
    ...rest
  } = props;

  const showIcon = isLoading || isValid || isInvalid;

  return (
    <div className={cn('relative', wrapperClassName)}>
      <label
        className={cn(
          'flex h-9 w-full cursor-text items-center rounded-md border border-border bg-surface px-3 py-2 text-sm text-text transition-colors focus-within:border-text-secondary',
          className
        )}
      >
        {prefix && (
          <span className="mr-px select-none text-text-tertiary">{prefix}</span>
        )}
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 bg-transparent outline-none placeholder:text-text-tertiary"
          {...rest}
        />
      </label>
      {showIcon && (
        <div className="absolute top-1/2 right-3 -translate-y-1/2">
          {isLoading && (
            <Loader2 className="size-4 animate-spin text-gray-400" />
          )}
          {isValid && <Check className="size-4 text-green-500" />}
          {isInvalid && <X className="size-4 text-red-500" />}
        </div>
      )}
    </div>
  );
}
