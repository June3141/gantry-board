import { useEffect, useRef } from 'react';

interface AgentOutputViewerProps {
  lines: string[];
}

export function AgentOutputViewer({ lines }: AgentOutputViewerProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  const lastLine = lines.length > 0 ? lines[lines.length - 1] : undefined;

  useEffect(() => {
    const container = bottomRef.current?.parentElement;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [lastLine]);

  if (lines.length === 0) {
    return (
      <div className="rounded-md bg-gray-50 p-4 text-sm text-gray-500">
        No output yet
      </div>
    );
  }

  return (
    <div
      data-testid="agent-output-container"
      className="max-h-64 overflow-y-auto rounded-md bg-gray-900 p-3 font-mono text-xs text-green-400"
    >
      {lines.map((line, i) => (
        <div key={`${i}-${line.slice(0, 20)}`}>{line}</div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
