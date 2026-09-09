import { useCallback, useEffect, useRef, useState } from 'react';
import type { Terminal } from '@xterm/xterm';
import type { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import {
  Maximize2,
  Minimize2,
  RotateCcw,
  SquareTerminal,
  Trash2,
} from 'lucide-react';
import { Button } from '~/components/interface/button';
import { HStack } from '~/components/interface/stacks';
import { TabActions } from '~/components/interface/tab-actions';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '~/components/interface/tooltip';
import { cn } from '~/utils/classname';
import { getAuthToken } from '~/utils/jwt';
import type { NodeWithInfo } from '~/queries/nodes';

export type ClientConsoleMessage =
  | { type: 'input'; data: string }
  | { type: 'resize'; cols: number; rows: number };

export type ServerConsoleMessage =
  | { type: 'output'; data: string }
  | { type: 'exit'; code: number | null }
  | { type: 'error'; message: string };

type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

type NodeConsoleProps = {
  node: NodeWithInfo;
};

export function NodeConsole({ node }: NodeConsoleProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const lastDimensions = useRef<{ cols: number; rows: number }>({
    cols: 80,
    rows: 24,
  });

  const [status, setStatus] = useState<ConnectionStatus>('connecting');
  const [isFullscreen, setIsFullscreen] = useState(false);

  const nodeTarget =
    node.id === 'local'
      ? 'local'
      : `${node.user || 'root'}@${node.host}:${node.port || 22}`;

  const connect = useCallback(() => {
    const term = terminalRef.current;
    if (!term) return;

    if (wsRef.current) {
      const existing = wsRef.current;
      existing.onopen = null;
      existing.onmessage = null;
      existing.onerror = null;
      existing.onclose = null;
      existing.close();
      wsRef.current = null;
    }

    setStatus('connecting');
    term.writeln('\x1b[90mConnecting to node console...\x1b[0m');

    if (fitAddonRef.current) {
      try {
        fitAddonRef.current.fit();
      } catch {
        // ignore layout resize
      }
    }

    lastDimensions.current = {
      cols: term.cols || 80,
      rows: term.rows || 24,
    };

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const token = getAuthToken();

    const queryParams = new URLSearchParams();
    if (token) queryParams.set('token', token);
    queryParams.set('cols', String(lastDimensions.current.cols));
    queryParams.set('rows', String(lastDimensions.current.rows));

    const wsUrl = `${protocol}//${host}/api/nodes/${node.id}/console?${queryParams.toString()}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus('connected');
      term.writeln(
        `\x1b[32m✔\x1b[0m Connected to \x1b[1m${node.name}\x1b[0m (${nodeTarget})`
      );
      term.focus();
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as ServerConsoleMessage;
        switch (msg.type) {
          case 'output':
            term.write(msg.data);
            break;
          case 'exit':
            setStatus('disconnected');
            term.writeln(
              `\r\n\x1b[90m[Process finished with exit code ${
                msg.code ?? 'unknown'
              }]\x1b[0m`
            );
            break;
          case 'error':
            setStatus('error');
            term.writeln(`\r\n\x1b[31m[Error: ${msg.message}]\x1b[0m`);
            break;
        }
      } catch {
        // ignore non-json messages
      }
    };

    ws.onerror = () => {
      setStatus('error');
      term.writeln('\r\n\x1b[31m[WebSocket connection error]\x1b[0m');
    };

    ws.onclose = () => {
      setStatus('disconnected');
      term.writeln('\r\n\x1b[90m[Connection closed]\x1b[0m');
    };
  }, [node.id, node.name, nodeTarget]);

  const handleReconnect = useCallback(() => {
    if (status === 'connecting') return;
    connect();
  }, [connect, status]);

  const handleClear = useCallback(() => {
    if (terminalRef.current) {
      terminalRef.current.clear();
      terminalRef.current.focus();
    }
  }, []);

  const toggleFullscreen = useCallback(() => {
    setIsFullscreen((prev) => !prev);
  }, []);

  useEffect(() => {
    if (!isFullscreen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsFullscreen(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isFullscreen]);

  useEffect(() => {
    if (!containerRef.current) return;

    let isMounted = true;
    let termInstance: Terminal | null = null;
    let resizeObserverInstance: ResizeObserver | null = null;
    let dataDisposableInstance: { dispose: () => void } | null = null;

    async function initTerminal() {
      const [{ Terminal }, { FitAddon }, { WebLinksAddon }] = await Promise.all(
        [
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
          import('@xterm/addon-web-links'),
        ]
      );

      if (!isMounted || !containerRef.current) return;

      const term = new Terminal({
        cursorBlink: true,
        cursorStyle: 'block',
        fontFamily:
          "'Geist Mono', 'Symbols Nerd Font Mono', 'Symbols Nerd Font', 'JetBrainsMono Nerd Font', 'FiraCode Nerd Font', 'MesloLGS NF', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        fontSize: 13,
        lineHeight: 1.35,
        letterSpacing: 0,
        scrollback: 10000,
        convertEol: true,
        theme: {
          background: '#0d0d0d',
          foreground: '#e5e5e5',
          cursor: '#ffffff',
          cursorAccent: '#111111',
          selectionBackground: 'rgba(255, 255, 255, 0.15)',
          selectionForeground: '#ffffff',
          black: '#1a1a1a',
          red: '#f87171',
          green: '#4ade80',
          yellow: '#facc15',
          blue: '#60a5fa',
          magenta: '#c084fc',
          cyan: '#38bdf8',
          white: '#f3f4f6',
          brightBlack: '#6b7280',
          brightRed: '#ef4444',
          brightGreen: '#22c55e',
          brightYellow: '#eab308',
          brightBlue: '#3b82f6',
          brightMagenta: '#a855f7',
          brightCyan: '#06b6d4',
          brightWhite: '#ffffff',
        },
      });

      const fitAddon = new FitAddon();
      const webLinksAddon = new WebLinksAddon();

      term.loadAddon(fitAddon);
      term.loadAddon(webLinksAddon);

      containerRef.current.innerHTML = '';
      term.open(containerRef.current);
      terminalRef.current = term;
      fitAddonRef.current = fitAddon;
      termInstance = term;

      term.attachCustomKeyEventHandler((event: KeyboardEvent) => {
        if (
          event.type === 'keydown' &&
          (event.ctrlKey || event.metaKey) &&
          event.key.toLowerCase() === 'c' &&
          term.hasSelection()
        ) {
          navigator.clipboard?.writeText(term.getSelection());
          return false;
        }
        if (
          event.type === 'keydown' &&
          (event.ctrlKey || event.metaKey) &&
          event.key.toLowerCase() === 'v'
        ) {
          navigator.clipboard
            ?.readText()
            .then((clipText) => {
              if (clipText && wsRef.current?.readyState === WebSocket.OPEN) {
                wsRef.current.send(
                  JSON.stringify({ type: 'input', data: clipText })
                );
              }
            })
            .catch(() => {});
          return false;
        }
        return true;
      });

      const dataDisposable = term.onData((data: string) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
          wsRef.current.send(JSON.stringify({ type: 'input', data }));
        }
      });
      dataDisposableInstance = dataDisposable;

      const resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(() => {
          try {
            if (!containerRef.current) return;
            fitAddon.fit();
            if (
              wsRef.current?.readyState === WebSocket.OPEN &&
              (term.cols !== lastDimensions.current.cols ||
                term.rows !== lastDimensions.current.rows)
            ) {
              lastDimensions.current = { cols: term.cols, rows: term.rows };
              wsRef.current.send(
                JSON.stringify({
                  type: 'resize',
                  cols: term.cols,
                  rows: term.rows,
                })
              );
            }
          } catch {
            // ignore layout calculations
          }
        });
      });

      resizeObserver.observe(containerRef.current);
      resizeObserverInstance = resizeObserver;

      connect();
    }

    initTerminal();

    return () => {
      isMounted = false;
      if (resizeObserverInstance) {
        resizeObserverInstance.disconnect();
      }
      if (dataDisposableInstance) {
        dataDisposableInstance.dispose();
      }
      if (wsRef.current) {
        wsRef.current.onopen = null;
        wsRef.current.onmessage = null;
        wsRef.current.onerror = null;
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
      if (termInstance) {
        termInstance.dispose();
      }
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [connect]);

  useEffect(() => {
    const handle = requestAnimationFrame(() => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        // ignore layout resize
      }
    });
    return () => cancelAnimationFrame(handle);
  }, [isFullscreen]);

  const statusBadge = (
    <div className="flex items-center gap-1.5 rounded-full border border-border bg-bg/50 px-2 py-0.5 text-[11px] font-medium">
      <span
        className={cn(
          'size-1.5 rounded-full',
          status === 'connected' && 'bg-emerald-500 animate-pulse',
          status === 'connecting' && 'bg-amber-500 animate-pulse',
          status === 'disconnected' && 'bg-text-tertiary',
          status === 'error' && 'bg-red-500'
        )}
      />
      <span
        className={cn(
          status === 'connected' && 'text-emerald-400',
          status === 'connecting' && 'text-amber-400',
          status === 'disconnected' && 'text-text-tertiary',
          status === 'error' && 'text-red-400'
        )}
      >
        {status === 'connected' && 'Connected'}
        {status === 'connecting' && 'Connecting...'}
        {status === 'disconnected' && 'Disconnected'}
        {status === 'error' && 'Error'}
      </span>
    </div>
  );

  return (
    <div
      className={cn(
        'flex flex-col bg-bg',
        isFullscreen
          ? 'fixed inset-0 z-50 h-screen w-screen p-4'
          : 'h-full min-h-0 flex-1 p-8'
      )}
    >
      {!isFullscreen && (
        <TabActions>
          <HStack space={2} alignItems="center">
            {statusBadge}
            {status !== 'connected' && (
              <Button
                size="sm"
                color="neutral"
                onClick={handleReconnect}
                icon={<RotateCcw className="size-3.5" />}
                label="Reconnect"
              />
            )}
            <Button
              size="sm"
              variant="ghost"
              color="neutral"
              onClick={handleClear}
              icon={<Trash2 className="size-3.5" />}
              label="Clear"
            />
            <Button
              size="sm"
              variant="ghost"
              color="neutral"
              onClick={toggleFullscreen}
              icon={<Maximize2 className="size-3.5" />}
              label="Fullscreen"
            />
          </HStack>
        </TabActions>
      )}

      <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-[#0d0d0d] shadow-2xl">
        <HStack
          justifyContent="between"
          alignItems="center"
          className="h-11 shrink-0 border-b border-border bg-surface/50 px-4 select-none"
        >
          <HStack space={3} alignItems="center">
            <SquareTerminal className="size-4 text-text-tertiary" />
            <span className="text-[12px] font-medium text-text">
              {node.name}
            </span>
            <span className="text-[11px] font-mono text-text-tertiary">
              {nodeTarget}
            </span>
            {isFullscreen && statusBadge}
          </HStack>

          <HStack space={1.5} alignItems="center">
            {status !== 'connected' && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    size="sm"
                    color="neutral"
                    onClick={handleReconnect}
                    icon={<RotateCcw className="size-3.5" />}
                  />
                </TooltipTrigger>
                <TooltipContent>Reconnect console</TooltipContent>
              </Tooltip>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  size="sm"
                  variant="ghost"
                  color="neutral"
                  onClick={handleClear}
                  icon={<Trash2 className="size-3.5" />}
                />
              </TooltipTrigger>
              <TooltipContent>Clear console output</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  size="sm"
                  variant="ghost"
                  color="neutral"
                  onClick={toggleFullscreen}
                  icon={
                    isFullscreen ? (
                      <Minimize2 className="size-3.5" />
                    ) : (
                      <Maximize2 className="size-3.5" />
                    )
                  }
                />
              </TooltipTrigger>
              <TooltipContent>
                {isFullscreen ? 'Exit Fullscreen (Esc)' : 'Toggle Fullscreen'}
              </TooltipContent>
            </Tooltip>
          </HStack>
        </HStack>

        <div
          ref={containerRef}
          onClick={() => terminalRef.current?.focus()}
          className="relative flex-1 min-h-0 w-full overflow-hidden p-2.5 outline-none cursor-text custom-scrollbar"
        />
      </div>
    </div>
  );
}
