import { Terminal } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '~/components/interface/dialog';
import { HStack } from '~/components/interface/stacks';
import { LogStream } from '~/components/apps/log-stream';

type LogStreamDialogProps = {
  url: string;
  title: string;
  onClose: () => void;
};

export function LogStreamDialog(props: LogStreamDialogProps) {
  const { url, title, onClose } = props;

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="flex h-[80vh] w-full max-w-4xl flex-col gap-0 border-border bg-bg p-0">
        <HStack
          justifyContent="between"
          className="shrink-0 border-b border-border p-4"
        >
          <HStack space={3}>
            <Terminal className="size-4 text-text-tertiary" />
            <DialogTitle className="text-sm">{title}</DialogTitle>
          </HStack>
        </HStack>

        <LogStream url={url} className="flex-1" />
      </DialogContent>
    </Dialog>
  );
}
