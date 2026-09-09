import { useEffect } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import {
  Activity,
  Bell,
  Box,
  Clock,
  Database,
  FileText,
  Gauge,
  History,
  KeyRound,
  Layers,
  LogOut,
  Plus,
  Server,
  Settings,
  User as UserIcon,
  Users,
  Webhook,
} from 'lucide-react';
import { LayoutGrid } from '~/components/icons/layout';
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from '~/components/interface/command';
import {
  getAlertChannelsOptions,
  getAlertRulesOptions,
} from '~/queries/alerts';
import { getAppsOptions } from '~/queries/apps';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodesOptions } from '~/queries/nodes';
import { getUsersOptions } from '~/queries/users';
import { removeAuthToken } from '~/utils/jwt';

type CommandMenuProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function CommandMenu(props: CommandMenuProps) {
  const { open, onOpenChange } = props;
  const navigate = useNavigate();
  const params = useParams();
  const { data: me } = useSuspenseQuery(getAuthMeOptions());
  const isAdmin = me.user?.role === 'Admin';
  const appSlug = params.slug;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'k' || !(event.metaKey || event.ctrlKey)) {
        return;
      }

      event.preventDefault();
      onOpenChange(!open);
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onOpenChange]);

  const { data: appsData } = useQuery({ ...getAppsOptions(), enabled: open });
  const { data: nodesData } = useQuery({
    ...getNodesOptions(),
    enabled: open && isAdmin,
  });
  const { data: rulesData } = useQuery({
    ...getAlertRulesOptions(),
    enabled: open && isAdmin,
  });
  const { data: channelsData } = useQuery({
    ...getAlertChannelsOptions(),
    enabled: open && isAdmin,
  });
  const { data: usersData } = useQuery({
    ...getUsersOptions(),
    enabled: open && isAdmin,
  });

  const run = (action: () => void) => {
    onOpenChange(false);
    action();
  };

  const go = (to: string) => {
    run(() => navigate(to));
  };

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Command menu"
      description="Search for a page, an app, or an action."
    >
      <CommandInput placeholder="Search pages, apps and actions..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {appSlug ? (
          <CommandGroup heading="This app">
            <CommandItem
              value={`Files ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}`)}
            >
              <FileText />
              Files
            </CommandItem>
            <CommandItem
              value={`Deployments ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/deployments`)}
            >
              <History />
              Deployments
            </CommandItem>
            <CommandItem
              value={`Scaling ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/scaling`)}
            >
              <Layers />
              Scaling
            </CommandItem>
            <CommandItem
              value={`Services ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/services`)}
            >
              <Database />
              Services
            </CommandItem>
            <CommandItem
              value={`Crons ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/crons`)}
            >
              <Clock />
              Crons
            </CommandItem>
            <CommandItem
              value={`Metrics ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/metrics`)}
            >
              <Activity />
              Metrics
            </CommandItem>
            <CommandItem
              value={`App settings ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/settings`)}
            >
              <Settings />
              App settings
            </CommandItem>
            <CommandItem
              value={`New cron job ${appSlug}`}
              onSelect={() => go(`/apps/${appSlug}/crons/new`)}
            >
              <Plus />
              New cron job
            </CommandItem>
          </CommandGroup>
        ) : null}

        <CommandGroup heading="Go to">
          <CommandItem onSelect={() => go('/apps')}>
            <LayoutGrid />
            Apps
          </CommandItem>
          {isAdmin ? (
            <CommandItem onSelect={() => go('/nodes')}>
              <Server />
              Nodes
            </CommandItem>
          ) : null}
          {isAdmin ? (
            <CommandItem onSelect={() => go('/alerts')}>
              <Bell />
              Alerts
            </CommandItem>
          ) : null}
          {isAdmin ? (
            <CommandItem onSelect={() => go('/users')}>
              <Users />
              Users
            </CommandItem>
          ) : null}
          <CommandItem onSelect={() => go('/settings/account')}>
            <Settings />
            Settings
          </CommandItem>
        </CommandGroup>

        {appsData?.apps.length ? (
          <CommandGroup heading="Apps">
            {appsData.apps.map((item) => (
              <CommandItem
                key={item.app.id}
                value={`${item.app.name} ${item.app.slug}`}
                onSelect={() => go(`/apps/${item.app.slug}`)}
              >
                <Box />
                {item.app.name}
                <CommandShortcut>{item.app.slug}</CommandShortcut>
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}

        {nodesData?.nodes.length ? (
          <CommandGroup heading="Nodes">
            {nodesData.nodes.map((node) => (
              <CommandItem
                key={node.id}
                value={`${node.name} node ${node.id}`}
                onSelect={() => go(`/nodes/${node.id}`)}
              >
                <Server />
                {node.name}
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}

        {rulesData?.rules.length ? (
          <CommandGroup heading="Alert rules">
            {rulesData.rules.map((rule) => (
              <CommandItem
                key={rule.id}
                value={`${rule.name} rule ${rule.id}`}
                onSelect={() => go(`/alerts/rules/${rule.id}/edit`)}
              >
                <Gauge />
                {rule.name}
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}

        {channelsData?.channels.length ? (
          <CommandGroup heading="Channels">
            {channelsData.channels.map((channel) => (
              <CommandItem
                key={channel.id}
                value={`${channel.name} channel ${channel.id}`}
                onSelect={() => go(`/alerts/channels/${channel.id}/edit`)}
              >
                <Webhook />
                {channel.name}
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}

        {usersData?.users.length ? (
          <CommandGroup heading="Users">
            {usersData.users.map((user) => (
              <CommandItem
                key={user.id}
                value={`${user.email} user ${user.id}`}
                onSelect={() => go(`/users/${user.id}/edit`)}
              >
                <UserIcon />
                {user.email}
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}

        <CommandGroup heading="Create">
          <CommandItem onSelect={() => go('/apps/new')}>
            <Plus />
            New app
          </CommandItem>
          {isAdmin ? (
            <CommandItem onSelect={() => go('/nodes/new')}>
              <Plus />
              Connect node
            </CommandItem>
          ) : null}
          {isAdmin ? (
            <CommandItem onSelect={() => go('/alerts/rules/new')}>
              <Plus />
              New alert rule
            </CommandItem>
          ) : null}
          {isAdmin ? (
            <CommandItem onSelect={() => go('/alerts/channels/new')}>
              <Plus />
              New channel
            </CommandItem>
          ) : null}
          {isAdmin ? (
            <CommandItem onSelect={() => go('/users/new')}>
              <Plus />
              Add user
            </CommandItem>
          ) : null}
          <CommandItem onSelect={() => go('/settings/ssh-keys/new')}>
            <KeyRound />
            Add SSH key
          </CommandItem>
        </CommandGroup>

        <CommandGroup heading="Account">
          <CommandItem
            value="Log out sign out"
            onSelect={() =>
              run(() => {
                removeAuthToken();
                navigate('/login');
              })
            }
          >
            <LogOut />
            Log out
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
