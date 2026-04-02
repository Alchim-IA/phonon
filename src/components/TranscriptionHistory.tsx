import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranscriptionStore } from '../stores/transcriptionStore';
import { useSettingsStore } from '../stores/settingsStore';
import { LlmProvider, TranslationEntry } from '../types';

interface SummaryState {
  [key: number]: {
    loading: boolean;
    text: string | null;
    error: string | null;
    durationMs: number | null;
  };
}

type HistoryItem =
  | { type: 'transcription'; data: { text: string; confidence: number; duration_seconds: number; processing_time_ms: number; detected_language: string | null; timestamp: number; model_used: string | null } }
  | { type: 'translation'; data: TranslationEntry };

export function TranscriptionHistory() {
  const { history, loadHistory, clearHistory } = useTranscriptionStore();
  const settings = useSettingsStore(state => state.settings);
  const [summaries, setSummaries] = useState<SummaryState>({});
  const [localLlmAvailable, setLocalLlmAvailable] = useState(false);
  const [hasGroqKey, setHasGroqKey] = useState(false);
  const [translationHistory, setTranslationHistory] = useState<TranslationEntry[]>([]);

  useEffect(() => {
    loadHistory();
    invoke<TranslationEntry[]>('get_translation_history')
      .then(setTranslationHistory)
      .catch(() => setTranslationHistory([]));
  }, [loadHistory]);

  // Vérifier si au moins un modèle LLM local est disponible
  useEffect(() => {
    invoke<string[]>('get_available_llm_models')
      .then(models => setLocalLlmAvailable(models.length > 0))
      .catch(() => setLocalLlmAvailable(false));
  }, [settings?.local_llm_model]);

  // Vérifier si une clé Groq est configurée
  useEffect(() => {
    invoke<boolean>('has_groq_api_key')
      .then(setHasGroqKey)
      .catch(() => setHasGroqKey(false));
  }, [settings?.groq_api_key]);

  const handleSummarize = useCallback(async (index: number, text: string, provider?: LlmProvider) => {
    setSummaries(prev => ({
      ...prev,
      [index]: { loading: true, text: null, error: null, durationMs: null }
    }));

    const start = performance.now();
    try {
      const summary = await invoke<string>('summarize_text_smart', { text, provider });
      const durationMs = Math.round(performance.now() - start);
      setSummaries(prev => ({
        ...prev,
        [index]: { loading: false, text: summary, error: null, durationMs }
      }));
    } catch (e) {
      setSummaries(prev => ({
        ...prev,
        [index]: { loading: false, text: null, error: String(e), durationMs: null }
      }));
    }
  }, []);

  const handleCopySummary = useCallback((text: string) => {
    navigator.clipboard.writeText(text);
  }, []);

  const handleSendTo = useCallback(async (target: 'apple_notes' | 'obsidian', text: string, timestamp: number) => {
    const title = `Transcription ${new Date(timestamp * 1000).toLocaleDateString('fr-FR')}`;
    try {
      if (target === 'apple_notes') {
        await invoke('send_to_apple_notes', { title, body: text });
      } else {
        await invoke('send_to_obsidian', { title, body: text });
      }
    } catch (e) {
      console.error(`Failed to send to ${target}:`, e);
    }
  }, []);

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('fr-FR', {
      day: '2-digit',
      month: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  // Fusionner et trier par timestamp décroissant
  const mergedHistory: HistoryItem[] = [
    ...history.map(item => ({ type: 'transcription' as const, data: item })),
    ...translationHistory.map(item => ({ type: 'translation' as const, data: item })),
  ].sort((a, b) => b.data.timestamp - a.data.timestamp);

  const handleClearAll = async () => {
    await clearHistory();
    try {
      await invoke('clear_translation_history');
      setTranslationHistory([]);
    } catch (e) {
      console.error('Failed to clear translation history:', e);
    }
  };

  if (mergedHistory.length === 0) {
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
            {mergedHistory.length}
          </span>
        </div>
        <button
          onClick={handleClearAll}
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
        {mergedHistory.map((entry, index) => {
          if (entry.type === 'translation') {
            const item = entry.data;
            return (
              <div
                key={`tr-${item.timestamp}-${index}`}
                className="result-card-frost cursor-default"
              >
                <div className="card-header">
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 rounded-full bg-gradient-to-br from-[#007AFF] to-[#00C7BE]" />
                    <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                      {formatDate(item.timestamp)}
                    </span>
                    <span className="tag-frost text-[0.6rem]" style={{ color: '#007AFF', borderColor: 'rgba(0,122,255,0.3)' }}>
                      Traduction
                    </span>
                    <span className="tag-frost text-[0.6rem]">
                      {item.provider === 'groq' ? 'Cloud' : 'Local'}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[0.65rem] text-[var(--text-muted)] tabular-nums">
                      {item.char_count} car.
                    </span>
                    <span className="text-[0.65rem] text-[var(--text-muted)] opacity-70 tabular-nums">
                      ⚡ {item.translation_time_ms >= 1000
                        ? `${(item.translation_time_ms / 1000).toFixed(1)}s`
                        : `${item.translation_time_ms}ms`}
                    </span>
                    <span className="text-[0.6rem] text-[var(--text-muted)] opacity-50 tabular-nums">
                      ({Math.round(item.char_count / (item.translation_time_ms / 1000))} car/s)
                    </span>
                  </div>
                </div>

                <div className="card-content space-y-2">
                  {/* Source */}
                  <div className="p-2.5 rounded-lg bg-[rgba(255,255,255,0.04)] border border-[rgba(255,255,255,0.06)]">
                    <div className="flex items-center gap-1.5 mb-1">
                      <span className="text-[0.6rem] font-medium text-[var(--text-muted)] uppercase tracking-wider">Source</span>
                    </div>
                    <p className="text-[var(--text-secondary)] text-[0.8rem] leading-relaxed line-clamp-2">
                      {item.source_text}
                    </p>
                  </div>
                  {/* Result */}
                  <div className="p-2.5 rounded-lg bg-[rgba(0,122,255,0.06)] border border-[rgba(0,122,255,0.15)]">
                    <div className="flex items-center gap-1.5 mb-1">
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#007AFF" strokeWidth="2">
                        <polyline points="5 12 10 17 20 7" />
                      </svg>
                      <span className="text-[0.6rem] font-medium text-[#007AFF] uppercase tracking-wider">
                        {item.target_language.toUpperCase()}
                      </span>
                    </div>
                    <p className="text-[var(--text-primary)] text-[0.8rem] leading-relaxed">
                      {item.translated_text}
                    </p>
                  </div>
                </div>
              </div>
            );
          }

          const item = entry.data;
          return (
            <div
              key={`tc-${item.timestamp}-${index}`}
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
                <div className="flex items-center gap-2">
                  {/* Bouton résumé */}
                  {summaries[index]?.loading ? (
                    <button
                      disabled
                      className="btn-glass text-[0.7rem] py-1 px-2 opacity-50"
                    >
                      <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                    </button>
                  ) : localLlmAvailable && hasGroqKey ? (
                    <div className="relative group">
                      <button className="btn-glass text-[0.7rem] py-1 px-2 flex items-center gap-1">
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                          <line x1="16" y1="13" x2="8" y2="13" />
                          <line x1="16" y1="17" x2="8" y2="17" />
                        </svg>
                        <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <polyline points="6 9 12 15 18 9" />
                        </svg>
                      </button>
                      <div className="absolute top-full right-0 mt-1 py-1 min-w-[120px] bg-[var(--glass-bg)] backdrop-blur-xl border border-[var(--glass-border)] rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
                        <button
                          onClick={() => handleSummarize(index, item.text, 'local')}
                          className="w-full px-3 py-1.5 text-left text-[0.7rem] text-[var(--text-secondary)] hover:bg-[rgba(255,255,255,0.08)] flex items-center gap-2"
                        >
                          <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                          Local
                        </button>
                        <button
                          onClick={() => handleSummarize(index, item.text, 'groq')}
                          className="w-full px-3 py-1.5 text-left text-[0.7rem] text-[var(--text-secondary)] hover:bg-[rgba(255,255,255,0.08)] flex items-center gap-2"
                        >
                          <span className="w-1.5 h-1.5 rounded-full bg-blue-500" />
                          Cloud
                        </button>
                      </div>
                    </div>
                  ) : (localLlmAvailable || hasGroqKey) ? (
                    <button
                      onClick={() => handleSummarize(index, item.text)}
                      className="btn-glass text-[0.7rem] py-1 px-2"
                      title={localLlmAvailable ? 'Resume (local)' : 'Resume (cloud)'}
                    >
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                        <line x1="16" y1="13" x2="8" y2="13" />
                        <line x1="16" y1="17" x2="8" y2="17" />
                      </svg>
                    </button>
                  ) : null}
                  <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                    {item.duration_seconds.toFixed(1)}s
                  </span>
                  {item.processing_time_ms > 0 && (
                    <span className="text-[0.65rem] text-[var(--text-muted)] opacity-70 tabular-nums">
                      ⚡ {item.processing_time_ms}ms
                    </span>
                  )}
                  {(settings?.integrations?.apple_notes_enabled || settings?.integrations?.obsidian_enabled) && (
                    <div className="relative group">
                      <button className="btn-glass text-[0.7rem] py-1 px-2">
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8" />
                          <polyline points="16 6 12 2 8 6" />
                          <line x1="12" y1="2" x2="12" y2="15" />
                        </svg>
                      </button>
                      <div className="absolute top-full right-0 mt-1 py-1 min-w-[140px] bg-[var(--glass-bg)] backdrop-blur-xl border border-[var(--glass-border)] rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
                        {settings?.integrations?.apple_notes_enabled && (
                          <button
                            onClick={() => handleSendTo('apple_notes', item.text, item.timestamp)}
                            className="w-full px-3 py-1.5 text-left text-[0.7rem] text-[var(--text-secondary)] hover:bg-[rgba(255,255,255,0.08)]"
                          >
                            Apple Notes
                          </button>
                        )}
                        {settings?.integrations?.obsidian_enabled && (
                          <button
                            onClick={() => handleSendTo('obsidian', item.text, item.timestamp)}
                            className="w-full px-3 py-1.5 text-left text-[0.7rem] text-[var(--text-secondary)] hover:bg-[rgba(255,255,255,0.08)]"
                          >
                            Obsidian
                          </button>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Item content */}
              <div className="card-content space-y-3">
                <p className="text-[var(--text-primary)] text-[0.9375rem] leading-relaxed line-clamp-3">
                  {item.text}
                </p>

                {/* Erreur de résumé */}
                {summaries[index]?.error && (
                  <div className="p-2 rounded-lg bg-[var(--accent-danger-soft)] border border-[var(--accent-danger)]">
                    <p className="text-[0.75rem] text-[var(--accent-danger)]">{summaries[index].error}</p>
                  </div>
                )}

                {/* Affichage du résumé */}
                {summaries[index]?.text && (
                  <div className="p-3 rounded-xl bg-[rgba(139,92,246,0.08)] border border-[var(--accent-primary-soft)]">
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="2">
                          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                          <line x1="16" y1="13" x2="8" y2="13" />
                          <line x1="16" y1="17" x2="8" y2="17" />
                        </svg>
                        <span className="text-[0.7rem] font-medium text-[var(--accent-primary)]">Resume</span>
                        {summaries[index].durationMs != null && (
                          <span className="text-[0.6rem] text-[var(--text-muted)] tabular-nums">
                            {summaries[index].durationMs! >= 1000
                              ? `${(summaries[index].durationMs! / 1000).toFixed(1)}s`
                              : `${summaries[index].durationMs}ms`}
                          </span>
                        )}
                      </div>
                      <button
                        onClick={() => handleCopySummary(summaries[index].text!)}
                        className="text-[var(--text-muted)] hover:text-[var(--accent-primary)] transition-colors"
                        title="Copier le resume"
                      >
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                        </svg>
                      </button>
                    </div>
                    <p className="text-[var(--text-primary)] text-[0.8rem] leading-relaxed">
                      {summaries[index].text}
                    </p>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
