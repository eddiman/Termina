import { useState, useEffect, useRef } from 'preact/hooks';
import { api, type LogLine } from '../lib/api';

interface Props {
  commandId: string;
  fullSize?: boolean;
}

export function LogViewer({ commandId, fullSize }: Props) {
  const [logs, setLogs] = useState<LogLine[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const autoScrollRef = useRef(true);

  useEffect(() => {
    let active = true;

    const poll = async () => {
      if (!active) return;
      try {
        const lines = await api.getLogs(commandId);
        setLogs(lines);
      } catch {
        // ignore
      }
      if (active) {
        setTimeout(poll, 1500);
      }
    };

    poll();

    return () => {
      active = false;
    };
  }, [commandId]);

  useEffect(() => {
    if (autoScrollRef.current && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const el = containerRef.current;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 30;
    autoScrollRef.current = atBottom;
  };

  return (
    <div class={`log-viewer ${fullSize ? 'log-viewer-full' : ''}`} ref={containerRef} onScroll={handleScroll}>
      {logs.length === 0 ? (
        <div class="log-empty">No output yet</div>
      ) : (
        logs.map((line, i) => (
          <div key={i} class={`log-line ${line.stream === 'stderr' ? 'log-stderr' : ''}`}>
            {line.text}
          </div>
        ))
      )}
    </div>
  );
}
