import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';
import type { AlertChannel } from '~/models/alerts';

export function ChannelMultiSelect(props: {
  channels: AlertChannel[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
}) {
  const selectedChannels = props.channels.filter((channel) =>
    props.selectedIds.includes(channel.id)
  );

  return (
    <div className="space-y-3">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className="flex min-h-10 w-full items-center justify-between rounded-md border border-border bg-surface px-3 py-2 text-left text-sm text-text transition-colors hover:border-text-secondary"
          >
            <span className="truncate">
              {selectedChannels.length > 0
                ? `${selectedChannels.length} channel${selectedChannels.length === 1 ? '' : 's'} selected`
                : 'Select channels'}
            </span>
            <span className="text-xs text-text-tertiary">Multi-select</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="start"
          className="w-[var(--radix-dropdown-menu-trigger-width)]"
        >
          {props.channels.length === 0 ? (
            <div className="px-2 py-2 text-xs text-text-tertiary">
              Add a channel first, then select it here.
            </div>
          ) : (
            props.channels.map((channel) => {
              const checked = props.selectedIds.includes(channel.id);

              return (
                <DropdownMenuCheckboxItem
                  key={channel.id}
                  checked={checked}
                  onSelect={(event) => event.preventDefault()}
                  onCheckedChange={(nextChecked) =>
                    props.onChange(
                      nextChecked
                        ? [...props.selectedIds, channel.id]
                        : props.selectedIds.filter((id) => id !== channel.id)
                    )
                  }
                  className="items-start py-2"
                >
                  <div className="min-w-0">
                    <div className="truncate font-medium text-text">
                      {channel.name}
                    </div>
                    <div className="truncate text-[11px] capitalize text-text-tertiary">
                      {channel.config.kind}
                    </div>
                  </div>
                </DropdownMenuCheckboxItem>
              );
            })
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      {selectedChannels.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {selectedChannels.map((channel) => (
            <span
              key={channel.id}
              className="inline-flex items-center rounded-full border border-border bg-bg/60 px-2.5 py-1 text-xs text-text-secondary"
            >
              {channel.name}
            </span>
          ))}
        </div>
      ) : (
        <p className="text-xs text-text-tertiary">No channels selected.</p>
      )}
    </div>
  );
}
