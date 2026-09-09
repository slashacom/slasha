import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '~/utils/classname';

type CopyButtonProps = {
  value: string;
  label?: string;
  className?: string;
};

export function CopyButton(props: CopyButtonProps) {
  const { value, label = 'Copy', className } = props;
  const [copied, setCopied] = useState(false);

  const handleCopy = async (event: React.MouseEvent) => {
    event.stopPropagation();

    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => {
        setCopied(false);
      }, 1500);
    } catch (e) {
      toast.error('Failed to copy: ' + e);
    }
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      aria-label={label}
      className={cn(
        'flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-text-tertiary transition-colors hover:bg-white/5 hover:text-text',
        className
      )}
    >
      {copied ? (
        <Check className="size-3 text-emerald-400" />
      ) : (
        <Copy className="size-3" />
      )}
    </button>
  );
}
