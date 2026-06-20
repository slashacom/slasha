import * as React from 'react';
import { useState, useCallback } from 'react';
import { EmojiPicker } from '@ferrucc-io/emoji-picker';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '~/components/interface/popover';
import { Button } from '~/components/interface/variant-button';
import { Hash } from 'lucide-react';

type EmojiPickerComponentProps = Omit<
  React.ComponentPropsWithoutRef<typeof PopoverTrigger>,
  'onSelect' | 'onOpenChange'
> & {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  triggerPlaceholder?: string;
  modal?: boolean;
  width?: number;
  height?: number;
  emojisPerRow?: number;
  emojiSize?: number;
}

const EmojiPickerComponent = React.forwardRef<
  React.ComponentRef<typeof PopoverTrigger>,
  EmojiPickerComponentProps
>(
  (
    {
      value,
      defaultValue,
      onValueChange,
      open,
      defaultOpen,
      onOpenChange,
      children,
      triggerPlaceholder = 'Select an emoji',
      modal = false,
      width = 320,
      height = 280,
      emojisPerRow = 7,
      emojiSize = 28,
      ...props
    },
    ref
  ) => {
    const [selectedEmoji, setSelectedEmoji] = useState<string | undefined>(
      defaultValue
    );
    const [isOpen, setIsOpen] = useState(defaultOpen || false);

    const handleValueChange = useCallback(
      (emoji: string) => {
        if (value === undefined) {
          setSelectedEmoji(emoji);
        }
        onValueChange?.(emoji);
      },
      [value, onValueChange]
    );

    const handleOpenChange = useCallback(
      (newOpen: boolean) => {
        if (open === undefined) {
          setIsOpen(newOpen);
        }
        onOpenChange?.(newOpen);
      },
      [open, onOpenChange]
    );

    const handleEmojiSelect = useCallback(
      (emoji: string) => {
        handleValueChange(emoji);
        setIsOpen(false);
      },
      [handleValueChange]
    );

    return (
      <Popover
        open={open ?? isOpen}
        onOpenChange={handleOpenChange}
        modal={modal}
      >
        <PopoverTrigger ref={ref} asChild {...props}>
          {children || (
            <Button variant="outline">
              {value || selectedEmoji ? (
                <span className="text-lg">{value || selectedEmoji}</span>
              ) : (
                <Hash className="size-3 text-neutral-400" />
              )}
            </Button>
          )}
        </PopoverTrigger>
        <PopoverContent
          align="start"
          className="p-0 overflow-hidden border border-gray-200 shadow-none outline-none focus:shadow-none focus:ring-0 bg-white w-autol focus:outline-none"
        >
          <div className="w-full">
            <EmojiPicker
              onEmojiSelect={handleEmojiSelect}
              emojisPerRow={7}
              emojiSize={28}
              className="w-full focus:ring-0 border-0"
            >
              <EmojiPicker.Header className="px-3 pt-3 bg-white">
                <EmojiPicker.Input
                  placeholder="Search emoji"
                  className="focus:ring-neutral-200 focus:ring-offset-neutral-200 focus:ring-offset-1 border-0"
                />
              </EmojiPicker.Header>
              <EmojiPicker.Group className="max-h-64 overflow-y-auto p-2">
                <EmojiPicker.List />
              </EmojiPicker.Group>
            </EmojiPicker>
          </div>
        </PopoverContent>
      </Popover>
    );
  }
);
EmojiPickerComponent.displayName = 'EmojiPicker';

export { EmojiPickerComponent as EmojiPicker };
