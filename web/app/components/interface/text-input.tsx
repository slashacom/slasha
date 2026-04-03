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
        className={`flex w-full cursor-text items-center rounded-lg border focus-within:border-gray-400/90 border-gray-300 px-3 py-2 text-sm ${className}`}
      >
        {prefix && (
          <span className="text-gray-500/80 select-none mr-px">{prefix}</span>
        )}
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 bg-transparent outline-none"
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
