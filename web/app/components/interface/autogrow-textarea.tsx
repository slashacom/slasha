import type { ChangeEvent } from 'react';
import TextareaAutosize from 'react-textarea-autosize';
import { cn } from '~/utils/classname';

type AutogrowTextareaProps = {
  id?: string;
  placeholder?: string;
  rows?: number;
  maxRows?: number;
  className?: string;
  autoFocus?: boolean;

  value: string;
  onValueChange: (value: string) => void;
};

export function AutogrowTextarea(props: AutogrowTextareaProps) {
  const { id, placeholder, value, onValueChange, className, autoFocus } = props;

  const handleInput = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const textarea = e.target;
    onValueChange(textarea.value);
  };

  return (
    <TextareaAutosize
      id={id}
      placeholder={placeholder}
      value={value}
      onChange={handleInput}
      autoFocus={autoFocus}
      className={cn(
        'flex min-h-[80px] w-full resize-none rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm placeholder:text-gray-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-500 focus-visible:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50',
        className
      )}
    />
  );
}
