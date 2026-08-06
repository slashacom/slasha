import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useReducer,
} from 'react';
import {
  ArrowDown,
  CircleDashed,
  Download,
  Search,
  X,
  Copy,
  Check,
} from 'lucide-react';
import { Virtuoso, type VirtuosoHandle } from 'react-virtuoso';
import { getAuthToken } from '~/utils/jwt';
import { cn } from '~/utils/classname';
import { parseUTC } from '~/utils/format';
import { Terminal } from 'lucide-react';
import type {
  LogRecord,
  LogStream,
  ResourceKind,
  LogPrefix,
} from '~/models/logs';

type LogStreamProps = {
  url: string;
  resourceKind: ResourceKind;
  className?: string;
  emptyMessage?: string;
  title?: string;
};

export function formatLogPrefix(prefix?: LogPrefix | null | string): string {
  if (!prefix) return '';
  if (typeof prefix === 'string') return prefix;
  if ('web' in prefix) return `web.${prefix.web}`;
  if ('worker' in prefix) return `worker.${prefix.worker}`;
  if ('custom' in prefix) return prefix.custom;
  return String(prefix);
}

type BasePrefix = Exclude<
  LogPrefix extends infer T ? (T extends string ? T : keyof T) : never,
  'custom'
>;

const PREFIX_BADGE_STYLES: Record<BasePrefix, string> = {
  system: 'bg-zinc-700/40 text-zinc-400',
  web: 'bg-emerald-500/10 text-emerald-400',
  worker: 'bg-purple-500/10 text-purple-400',
  service: 'bg-indigo-500/10 text-indigo-400',
};

function getPrefixBadge(prefix?: LogPrefix | null): string {
  if (!prefix) return 'bg-white/5 text-text-tertiary';
  const category = typeof prefix === 'string' ? prefix : Object.keys(prefix)[0];
  return (
    PREFIX_BADGE_STYLES[category as BasePrefix] ||
    'bg-white/5 text-text-tertiary'
  );
}

const ALLOWED_PREFIXES_BY_KIND: Record<ResourceKind, BasePrefix[]> = {
  deployment: ['system', 'web', 'worker'],
  service: ['system', 'service'],
  cron: ['system'],
  node: ['system'],
};

function formatLocalTime(isoString: string): string {
  try {
    const date = parseUTC(isoString);
    if (Number.isNaN(date.getTime())) return isoString;
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    const millis = date.getMilliseconds().toString().padStart(3, '0');
    return `${hours}:${minutes}:${seconds}.${millis}`;
  } catch {
    return isoString;
  }
}

function formatLogLine(log: LogRecord): string {
  const prefixStr = log.prefix ? ` [${formatLogPrefix(log.prefix)}]` : '';
  return `[${formatLocalTime(log.timestamp)}]${prefixStr} [${log.stream}] ${log.message}`;
}

function doesLogMatchFilter(
  log: LogRecord,
  search: string,
  selectedPrefix: string
): boolean {
  const q = search.trim().toLowerCase();
  const selPrefix = selectedPrefix.toLowerCase();

  if (selectedPrefix !== 'all') {
    const p = formatLogPrefix(log.prefix).toLowerCase();
    if (p !== selPrefix && !p.startsWith(`${selPrefix}.`)) {
      return false;
    }
  }

  if (q) {
    const msgMatch = log.message.toLowerCase().includes(q);
    const prefixMatch = formatLogPrefix(log.prefix).toLowerCase().includes(q);
    if (!msgMatch && !prefixMatch) return false;
  }

  return true;
}

function LogRow({
  log,
  onPrefixClick,
}: {
  log: LogRecord;
  onPrefixClick: (prefix: string) => void;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    const selection = window.getSelection();
    if (selection && selection.toString().length > 0) return;

    navigator.clipboard.writeText(formatLogLine(log));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      onClick={handleCopy}
      className="group relative flex items-start gap-3 rounded py-1 px-5 hover:bg-white/[0.06] transition-colors cursor-pointer"
    >
      {/* timestamp */}
      <span className="w-[84px] shrink-0 select-none font-mono text-[11px] text-text-tertiary mt-[1.5px]">
        {formatLocalTime(log.timestamp)}
      </span>

      {/* labels column */}
      <div className="w-40 shrink-0 flex items-start flex-wrap gap-1.5 mt-[1px]">
        {log.prefix ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onPrefixClick(formatLogPrefix(log.prefix) ?? 'all');
            }}
            title={`Click to filter logs by prefix: ${log.prefix}`}
            className={cn(
              'max-w-[100px] truncate px-1.5 py-0.5 rounded-sm text-[9px] font-medium tracking-wide uppercase cursor-pointer hover:brightness-125 transition-all',
              getPrefixBadge(log.prefix)
            )}
          >
            {formatLogPrefix(log.prefix)}
          </button>
        ) : null}
        <span
          className={cn(
            'select-none px-1.5 py-0.5 rounded-sm text-[9px] font-medium uppercase tracking-wider',
            log.stream === 'stderr'
              ? 'bg-red-500/10 text-red-400'
              : 'bg-white/5 text-text-tertiary'
          )}
        >
          {log.stream}
        </span>
      </div>

      {/* message */}
      <span
        className={cn(
          'min-w-0 flex-1 whitespace-pre-wrap break-words font-mono leading-relaxed',
          log.stream === 'stderr' ? 'text-red-400' : 'text-text'
        )}
      >
        {log.message}
      </span>

      {/* copy Indicator */}
      <div className="absolute right-4 top-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
        {copied ? (
          <span className="flex items-center gap-1.5 rounded bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-400 border border-emerald-500/20">
            <Check className="size-3" /> Copied
          </span>
        ) : (
          <span className="flex items-center gap-1.5 rounded bg-surface px-2 py-0.5 text-[10px] font-medium text-text-tertiary border border-border shadow-sm">
            <Copy className="size-3" /> Click to copy
          </span>
        )}
      </div>
    </div>
  );
}

type LogState = {
  logs: LogRecord[];
  firstItemIndex: number;
  hasMoreOlder: boolean;
  hasMoreNewer: boolean;
  liveBuffer: LogRecord[];
};

type LogAction =
  | { type: 'INIT'; logs: LogRecord[] }
  | { type: 'PREPEND_OLDER'; logs: LogRecord[] }
  | { type: 'APPEND_NEWER'; logs: LogRecord[] }
  | { type: 'APPEND_LIVE'; log: LogRecord };

function logReducer(state: LogState, action: LogAction): LogState {
  switch (action.type) {
    case 'INIT': {
      return {
        logs: action.logs,
        firstItemIndex: 1_000_000,
        hasMoreOlder: action.logs.length >= 2000,
        hasMoreNewer: false,
        liveBuffer: [],
      };
    }
    case 'PREPEND_OLDER': {
      if (action.logs.length === 0) return { ...state, hasMoreOlder: false };

      let newLogs = [...action.logs, ...state.logs];

      // virtuoso requires firstItemIndex to shift exactly by the number of prepended items
      let newIndex = state.firstItemIndex - action.logs.length;
      let hasMoreNewer = state.hasMoreNewer;

      // strictly enforce 2000 log cap to prevent memory bloat
      if (newLogs.length > 2000) {
        newLogs = newLogs.slice(0, 2000); // slice from the bottom
        hasMoreNewer = true; // we sliced off newest logs, meaning user is paged into the past
      }

      return {
        ...state,
        logs: newLogs,
        firstItemIndex: newIndex,
        hasMoreOlder: action.logs.length >= 1000,
        hasMoreNewer,
      };
    }
    case 'APPEND_NEWER': {
      let newLogs = [...state.logs, ...action.logs];
      let hasMoreNewer = action.logs.length >= 1000;

      // if we hit the absolute present (caught up with the db), flush the liveBuffer to
      // seal the race condition gap where an SSE log was emitted but not yet committed to the DB
      if (!hasMoreNewer) {
        const newestTs = newLogs[newLogs.length - 1]?.timestamp || '';
        const existingIds = new Set(newLogs.map((l) => l.id));
        const bufferedToAppend = state.liveBuffer.filter(
          (l) => l.timestamp >= newestTs && !existingIds.has(l.id)
        );
        newLogs = [...newLogs, ...bufferedToAppend];
      }

      let newIndex = state.firstItemIndex;
      let hasMoreOlder = state.hasMoreOlder;

      // strictly enforce 2000 log cap
      if (newLogs.length > 2000) {
        const sliceCount = newLogs.length - 2000;
        newIndex += sliceCount; // shift index down because we sliced from the top
        newLogs = newLogs.slice(sliceCount);
        hasMoreOlder = true;
      }

      return {
        ...state,
        logs: newLogs,
        firstItemIndex: newIndex,
        hasMoreNewer,
        hasMoreOlder,
      };
    }
    case 'APPEND_LIVE': {
      if (state.hasMoreNewer) {
        // user is viewing history, ignore live logs but keep them buffered
        // to seal the REST API pagination gap
        const newBuffer = [...state.liveBuffer, action.log].slice(-200);
        return { ...state, liveBuffer: newBuffer };
      }

      if (state.logs.some((l) => l.id === action.log.id)) {
        // deduplicate identical logs overlapping between REST API init and SSE stream
        // returning state directly avoids a useless re-render
        return state;
      }

      let newLogs = [...state.logs, action.log];
      let newIndex = state.firstItemIndex;
      let hasMoreOlder = state.hasMoreOlder;

      // strictly enforce 2000 log cap for live stream
      if (newLogs.length > 2000) {
        // increment index to account for shifting the array window
        newIndex += 1;
        newLogs = newLogs.slice(1);
        hasMoreOlder = true;
      }

      return {
        ...state,
        logs: newLogs,
        firstItemIndex: newIndex,
        hasMoreOlder,
      };
    }
  }
}

export function LogStream(props: LogStreamProps) {
  const { url, resourceKind, className, emptyMessage, title } = props;

  const [state, dispatch] = useReducer(logReducer, {
    logs: [],
    firstItemIndex: 1_000_000,
    hasMoreOlder: false,
    hasMoreNewer: false,
    liveBuffer: [],
  });

  const [loadingHistory, setLoadingHistory] = useState(true);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [loadingNewer, setLoadingNewer] = useState(false);

  const [search, setSearch] = useState('');
  const [selectedPrefix, setSelectedPrefix] = useState<string>('all');
  const [isAtBottom, setIsAtBottom] = useState(true);

  const virtuosoRef = useRef<VirtuosoHandle>(null);

  const searchRef = useRef(search);
  searchRef.current = search;
  const selectedPrefixRef = useRef(selectedPrefix);
  selectedPrefixRef.current = selectedPrefix;

  const allowedPrefixes = ALLOWED_PREFIXES_BY_KIND[resourceKind];

  const fetchLogs = useCallback(
    async (params: {
      limit: number;
      before_ts?: string;
      after_ts?: string;
    }) => {
      const queryParams = new URLSearchParams();
      queryParams.set('limit', params.limit.toString());
      if (params.before_ts) queryParams.set('before_ts', params.before_ts);
      if (params.after_ts) queryParams.set('after_ts', params.after_ts);

      queryParams.set('resource_kind', resourceKind);

      const q = search.trim();
      if (q) queryParams.set('search', q);

      const selPrefix = selectedPrefix.trim();
      if (selPrefix !== 'all') queryParams.set('prefix', selPrefix);

      const res = await fetch(`${url}/logs?${queryParams.toString()}`, {
        headers: { Authorization: `Bearer ${getAuthToken()}` },
      });
      if (!res.ok) throw res;
      const data = await res.json();
      return data?.logs && Array.isArray(data.logs)
        ? (data.logs as LogRecord[])
        : [];
    },
    [url, resourceKind, search, selectedPrefix]
  );

  // query backend history rest api on initial load or filter change
  useEffect(() => {
    let isSubscribed = true;

    const timeoutId = setTimeout(() => {
      setLoadingHistory(true);

      fetchLogs({ limit: 2000 })
        .then((fetchedLogs) => {
          if (isSubscribed) {
            dispatch({ type: 'INIT', logs: fetchedLogs });
          }
        })
        .catch(() => {})
        .finally(() => {
          if (isSubscribed) setLoadingHistory(false);
        });
    }, 300);

    return () => {
      isSubscribed = false;
      clearTimeout(timeoutId);
    };
  }, [fetchLogs]);

  // listen to live sse stream
  useEffect(() => {
    const token = getAuthToken();
    const queryParams = new URLSearchParams();
    if (token) queryParams.set('token', token);
    if (resourceKind) queryParams.set('resource_kind', resourceKind);

    const es = new EventSource(`${url}/stream?${queryParams.toString()}`);

    es.onmessage = (event) => {
      if (event.data) {
        const record: LogRecord = JSON.parse(event.data);
        if (
          doesLogMatchFilter(
            record,
            searchRef.current,
            selectedPrefixRef.current
          )
        ) {
          dispatch({ type: 'APPEND_LIVE', log: record });
        }
      }
    };

    es.onerror = () => es.close();

    return () => es.close();
  }, [url, resourceKind]);

  // handle upward scrolling pagination
  const loadOlderLogs = useCallback(() => {
    if (!state.hasMoreOlder || loadingOlder || state.logs.length === 0) return;

    setLoadingOlder(true);
    const oldestLog = state.logs[0];

    fetchLogs({ limit: 1000, before_ts: oldestLog.timestamp })
      .then((fetchedLogs) => {
        dispatch({ type: 'PREPEND_OLDER', logs: fetchedLogs });
      })
      .catch(() => {})
      .finally(() => setLoadingOlder(false));
  }, [state.hasMoreOlder, loadingOlder, state.logs, fetchLogs]);

  const loadNewerLogs = useCallback(() => {
    if (!state.hasMoreNewer || loadingNewer || state.logs.length === 0) return;

    setLoadingNewer(true);
    const newestLog = state.logs[state.logs.length - 1];

    fetchLogs({ limit: 1000, after_ts: newestLog.timestamp })
      .then((fetchedLogs) => {
        dispatch({ type: 'APPEND_NEWER', logs: fetchedLogs });
      })
      .catch(() => {})
      .finally(() => setLoadingNewer(false));
  }, [state.hasMoreNewer, loadingNewer, state.logs, fetchLogs]);

  const scrollToBottom = () => {
    virtuosoRef.current?.scrollToIndex({
      index: state.logs.length - 1,
      align: 'end',
      behavior: 'smooth',
    });
  };

  const handleDownloadLogs = () => {
    const queryParams = new URLSearchParams();
    queryParams.set('download', 'true');
    if (searchRef.current) queryParams.set('search', searchRef.current);
    if (selectedPrefixRef.current && selectedPrefixRef.current !== 'all') {
      queryParams.set('prefix', selectedPrefixRef.current);
    }
    if (resourceKind) queryParams.set('resource_kind', resourceKind);

    const token = getAuthToken();
    if (token) queryParams.set('token', token);

    const downloadUrl = `${url}/logs?${queryParams.toString()}`;

    const link = document.createElement('a');
    link.href = downloadUrl;
    link.download = '';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  return (
    <div className={cn('flex flex-col gap-3 h-full', className)}>
      <div className="flex shrink-0 flex-wrap items-center gap-3">
        {title ? (
          <div className="flex items-center gap-2 mr-auto">
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">{title}</h3>
          </div>
        ) : null}

        <div className="flex items-center gap-2 ml-auto">
          <div className="relative flex items-center">
            <Search className="absolute left-2.5 size-3.5 text-text-tertiary pointer-events-none" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search logs..."
              className="h-7 w-40 rounded border border-border bg-surface pl-8 pr-7 text-[11px] text-text placeholder:text-text-tertiary focus:border-text-secondary focus:outline-none focus:ring-1 focus:ring-text-secondary/50"
            />
            {search ? (
              <button
                type="button"
                onClick={() => setSearch('')}
                className="absolute right-2 text-text-tertiary hover:text-text"
              >
                <X className="size-3" />
              </button>
            ) : null}
          </div>

          <div className="relative flex items-center">
            <input
              type="text"
              value={selectedPrefix === 'all' ? '' : selectedPrefix}
              onChange={(e) =>
                setSelectedPrefix(e.target.value.trim() || 'all')
              }
              placeholder="Prefix (e.g. web.0)"
              list="log-prefix-suggestions"
              className="h-7 w-32 rounded border border-border bg-surface pl-2 pr-6 text-[10px] font-medium text-text placeholder:text-text-tertiary focus:border-text-secondary focus:outline-none"
            />
            <datalist id="log-prefix-suggestions">
              {allowedPrefixes.map((p) => {
                const formatted = formatLogPrefix(p);
                return <option key={formatted} value={formatted} />;
              })}
            </datalist>
            {selectedPrefix !== 'all' ? (
              <button
                type="button"
                onClick={() => setSelectedPrefix('all')}
                title="Clear prefix filter"
                className="absolute right-1.5 text-text-tertiary hover:text-text"
              >
                <X className="size-3" />
              </button>
            ) : null}
          </div>

          <div className="flex items-center gap-1 border-l border-border pl-2">
            <button
              type="button"
              onClick={handleDownloadLogs}
              title="Download logs"
              className="flex size-7 items-center justify-center rounded border border-border bg-surface text-text-tertiary transition-colors hover:bg-white/[0.06] hover:text-text"
            >
              <Download className="size-3" />
            </button>
          </div>
        </div>
      </div>

      <div className="relative flex-1 overflow-hidden bg-surface border border-border rounded-lg text-[12px] leading-relaxed selection:bg-sky-500/20">
        {state.logs.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 py-16 text-text-tertiary">
            {loadingHistory ? (
              <>
                <CircleDashed className="size-5 animate-spin text-text-tertiary" />
                <p className="text-sm font-medium text-text-secondary">
                  Fetching log history...
                </p>
              </>
            ) : (
              <p>
                {emptyMessage ||
                  'No log entries found. Waiting for new logs...'}
              </p>
            )}
          </div>
        ) : (
          <Virtuoso
            ref={virtuosoRef}
            data={state.logs}
            skipAnimationFrameInResizeObserver
            firstItemIndex={state.firstItemIndex}
            startReached={loadOlderLogs}
            endReached={loadNewerLogs}
            initialTopMostItemIndex={state.logs.length - 1}
            followOutput="smooth"
            atBottomStateChange={(atBottom) => setIsAtBottom(atBottom)}
            className="h-full w-full custom-scrollbar"
            itemContent={(index, log) => (
              <LogRow
                key={log.id}
                log={log}
                onPrefixClick={setSelectedPrefix}
              />
            )}
          />
        )}

        {!isAtBottom && state.logs.length > 0 ? (
          <button
            type="button"
            onClick={scrollToBottom}
            title="Scroll to latest"
            className="absolute bottom-6 right-8 flex size-8 items-center justify-center rounded-full border border-border bg-surface text-text-tertiary shadow-xl backdrop-blur transition-all hover:bg-white/[0.06] hover:text-text z-10"
          >
            <ArrowDown className="size-4" />
          </button>
        ) : null}
      </div>
    </div>
  );
}
