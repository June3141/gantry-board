import { useEffect, useRef } from 'react';

interface AgentOutputViewerProps {
  lines: string[];
  isLoading?: boolean;
}

export function AgentOutputViewer({ lines, isLoading = false }: AgentOutputViewerProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  const lineCount = lines.length;
  useEffect(() => {
    if (lineCount === 0) return;
    const container = bottomRef.current?.parentElement;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [lineCount]);

  if (isLoading) {
    return <div className="rounded-md bg-gray-50 p-4 text-sm text-gray-500">Loading output...</div>;
  }

  if (lines.length === 0) {
    return <div className="rounded-md bg-gray-50 p-4 text-sm text-gray-500">No output yet</div>;
  }

  return (
    <div
      data-testid="agent-output-container"
      className="max-h-64 overflow-y-auto rounded-md bg-gray-900 p-3 font-mono text-xs text-green-400"
    >
      {lines.map((line, i) => (
        <div key={`${i}-${line}`}>{line}</div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
