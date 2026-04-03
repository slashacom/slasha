import { GripVertical } from 'lucide-react';
import { Group, Panel, Separator } from 'react-resizable-panels';
import type { Orientation } from 'react-resizable-panels';
import { cn } from '~/utils/classname';

const ResizablePanelGroup = ({
  className,
  direction,
  ...props
}: Omit<React.ComponentProps<typeof Group>, 'orientation'> & {
  direction?: Orientation;
}) => (
  <Group
    orientation={direction}
    className={cn(
      'flex h-full w-full data-[panel-group-direction=vertical]:flex-col',
      className
    )}
    {...props}
  />
);

const ResizablePanel = Panel;

const ResizableHandle = ({
  withHandle,
  className,
  ...props
}: React.ComponentProps<typeof Separator> & {
  withHandle?: boolean;
}) => (
  <Separator
    className={cn(
      'relative flex w-px items-center justify-center bg-gray-200 after:absolute after:inset-y-0 after:left-1/2 after:w-1 after:-translate-x-1/2 data-[panel-group-direction=vertical]:h-px data-[panel-group-direction=vertical]:w-full data-[panel-group-direction=vertical]:after:left-0 data-[panel-group-direction=vertical]:after:h-1 data-[panel-group-direction=vertical]:after:w-full data-[panel-group-direction=vertical]:after:-translate-y-1/2 data-[panel-group-direction=vertical]:after:translate-x-0 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-offset-1 [&[data-panel-group-direction=vertical]>div]:rotate-90',
      className
    )}
    {...props}
  >
    {withHandle && (
      <div className="z-10 flex h-[30px] w-3 items-center justify-center rounded-sm bg-gray-200 text-black hover:bg-gray-300">
        <GripVertical className="size-5" />
      </div>
    )}
  </Separator>
);

export { ResizablePanelGroup, ResizablePanel, ResizableHandle };
