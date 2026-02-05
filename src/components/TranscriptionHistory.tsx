import { useEffect } from 'react';
import { useTranscriptionStore } from '../stores/transcriptionStore';

export function TranscriptionHistory() {
  const { history, loadHistory, clearHistory } = useTranscriptionStore();

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('fr-FR', {
      day: '2-digit',
      month: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (history.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-8 text-center animate-fade-in-up">
        <div className="w-20 h-20 rounded-3xl bg-[rgba(255,255,255,0.06)] backdrop-blur-xl border border-[var(--glass-border)] flex items-center justify-center mb-5 shadow-lg">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" strokeWidth="1.5">
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
          </svg>
        </div>
        <p className="text-[var(--text-secondary)] text-base font-medium mb-2">Aucun historique</p>
        <p className="text-[var(--text-muted)] text-sm">Les transcriptions apparaitront ici</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="flex-shrink-0 px-5 py-4 bg-[rgba(255,255,255,0.02)] border-b border-[rgba(255,255,255,0.06)] flex justify-between items-center">
        <div className="flex items-center gap-4">
          <span className="text-[0.875rem] text-[var(--text-secondary)] font-medium">
            Historique
          </span>
          <span className="tag-frost accent">
            {history.length}
          </span>
        </div>
        <button
          onClick={clearHistory}
          className="btn-glass text-[var(--accent-danger)] border-[var(--accent-danger-soft)] hover:bg-[var(--accent-danger-soft)]"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
          Effacer
        </button>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto p-5 space-y-4 scrollbar-thin stagger-children">
        {history.map((item, index) => (
          <div
            key={`${item.timestamp}-${index}`}
            className="result-card-frost cursor-default"
          >
            {/* Item header */}
            <div className="card-header">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)]" />
                <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                  {formatDate(item.timestamp)}
                </span>
                {item.model_used && (
                  <span className="tag-frost text-[0.6rem]">
                    {item.model_used}
                  </span>
                )}
              </div>
              <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                {item.duration_seconds.toFixed(1)}s
              </span>
            </div>

            {/* Item content */}
            <div className="card-content">
              <p className="text-[var(--text-primary)] text-[0.9375rem] leading-relaxed line-clamp-3">
                {item.text}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
