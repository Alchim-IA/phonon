import { useEffect, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { listen } from '@tauri-apps/api/event';
import { useTranscriptionStore } from '../stores/transcriptionStore';
import { useSettingsStore } from '../stores/settingsStore';
import { StreamingChunk } from '../types';

export function DictationPanel() {
  const { status, result, error, startRecording, stopRecording, clearError, setStatus } = useTranscriptionStore();
  const { settings } = useSettingsStore();
  const [streamingText, setStreamingText] = useState<string>('');
  const [recordingDuration, setRecordingDuration] = useState<number>(0);

  // Écouter les événements de statut PTT (push-to-talk)
  useEffect(() => {
    const unlistenStatus = listen<string>('recording-status', (event) => {
      const newStatus = event.payload as 'idle' | 'recording' | 'processing';
      setStatus(newStatus);
      if (newStatus === 'recording') {
        setStreamingText('');
        setRecordingDuration(0);
      }
    });

    return () => {
      unlistenStatus.then((fn) => fn());
    };
  }, [setStatus]);

  // Écouter les événements de streaming
  useEffect(() => {
    if (!settings?.streaming_enabled) return;

    const unlistenChunk = listen<StreamingChunk>('transcription-chunk', (event) => {
      const chunk = event.payload;
      if (chunk.is_final) {
        setStreamingText(chunk.text);
      } else {
        setStreamingText((prev) => prev + chunk.text);
      }
    });

    return () => {
      unlistenChunk.then((fn) => fn());
    };
  }, [settings?.streaming_enabled]);

  // Compteur de durée pendant l'enregistrement
  useEffect(() => {
    if (status !== 'recording') {
      return;
    }

    setStreamingText('');
    setRecordingDuration(0);

    const interval = setInterval(() => {
      setRecordingDuration((prev) => prev + 0.1);
    }, 100);

    return () => clearInterval(interval);
  }, [status]);

  const handleToggle = async () => {
    try {
      if (status === 'recording') {
        const transcription = await stopRecording();
        if (settings?.auto_copy_to_clipboard && transcription.text) {
          await writeText(transcription.text);
        }
      } else if (status === 'idle' || status === 'completed' || status === 'error') {
        await startRecording();
      }
    } catch (err) {
      console.error('Recording error:', err);
    }
  };

  const getStatusText = () => {
    switch (status) {
      case 'recording':
        return 'CAPTURE EN COURS';
      case 'processing':
        return 'ANALYSE NEURALE';
      case 'completed':
        return 'TRANSCRIPTION TERMINÉE';
      case 'error':
        return 'ERREUR SYSTÈME';
      default:
        return 'PRÊT À CAPTURER';
    }
  };

  return (
    <div className="h-full flex flex-col items-center justify-center p-8 gap-8">
      {/* Main record button */}
      <div className="relative">
        {/* Outer ring animation for recording */}
        {status === 'recording' && (
          <>
            <div className="absolute inset-0 rounded-full border-2 border-[var(--accent-red)] animate-ping opacity-30"
                 style={{ transform: 'scale(1.3)' }} />
            <div className="absolute inset-0 rounded-full border border-[var(--accent-red)] animate-pulse opacity-50"
                 style={{ transform: 'scale(1.15)' }} />
          </>
        )}

        {/* Outer ring animation for processing */}
        {status === 'processing' && (
          <>
            <div className="absolute inset-0 rounded-full border-2 border-[var(--accent-magenta)] animate-pulse opacity-40"
                 style={{ transform: 'scale(1.25)' }} />
            <div className="absolute inset-0 rounded-full border border-[var(--accent-cyan)] opacity-30"
                 style={{ transform: 'scale(1.4)', animation: 'spin 3s linear infinite' }} />
          </>
        )}

        <button
          onClick={handleToggle}
          disabled={status === 'processing'}
          className={`record-btn relative w-32 h-32 rounded-full flex items-center justify-center transition-all duration-300 ${
            status === 'recording' ? 'recording' : ''
          } ${status === 'processing' ? 'opacity-50 cursor-not-allowed' : ''}`}
          aria-label={status === 'recording' ? 'Arrêter' : 'Démarrer'}
        >
          {/* Inner content */}
          <div className="relative z-10">
            {status === 'processing' ? (
              <svg className="w-12 h-12 text-[var(--accent-cyan)] animate-spin" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2" strokeDasharray="31.416" strokeDashoffset="10" />
              </svg>
            ) : status === 'recording' ? (
              <div className="w-8 h-8 bg-[var(--accent-red)] rounded-sm" />
            ) : (
              <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-[var(--accent-cyan)]">
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                <line x1="12" x2="12" y1="19" y2="22" />
              </svg>
            )}
          </div>
        </button>
      </div>

      {/* Status display */}
      <div className="text-center space-y-2">
        <div className="flex items-center justify-center gap-2">
          <div className={`led ${status === 'recording' ? 'recording' : status === 'processing' ? 'processing' : 'active'}`} />
          <span className="text-[0.7rem] uppercase tracking-[0.2em] text-[var(--text-secondary)] font-medium">
            {getStatusText()}
          </span>
          {/* Badge LLM */}
          {settings?.llm_enabled && (
            <span className="px-1.5 py-0.5 text-[0.5rem] uppercase tracking-wider bg-[var(--accent-cyan)]/20 text-[var(--accent-cyan)] border border-[var(--accent-cyan)]/30 rounded">
              LLM
            </span>
          )}
        </div>

        {/* Waveform visualization and duration */}
        {status === 'recording' && (
          <div className="space-y-3">
            <div className="flex items-center justify-center gap-[2px] h-8">
              {[...Array(20)].map((_, i) => (
                <div
                  key={i}
                  className="w-[3px] bg-[var(--accent-cyan)] rounded-full waveform-bar"
                  style={{
                    animationDelay: `${i * 50}ms`,
                    height: '100%'
                  }}
                />
              ))}
            </div>
            <div className="text-[0.7rem] text-[var(--text-muted)] font-mono text-center">
              {recordingDuration.toFixed(1)}s
            </div>
          </div>
        )}

        {/* Processing indicator */}
        {status === 'processing' && (
          <div className="flex flex-col items-center gap-2">
            <div className="flex items-center justify-center gap-1 h-8">
              {[...Array(5)].map((_, i) => (
                <div
                  key={i}
                  className="w-2 h-2 bg-[var(--accent-magenta)] rounded-full animate-bounce"
                  style={{ animationDelay: `${i * 150}ms` }}
                />
              ))}
            </div>
            <span className="text-[0.6rem] text-[var(--text-muted)] uppercase tracking-wider">
              Transcription en cours...
            </span>
          </div>
        )}
      </div>

      {/* Streaming text display */}
      {settings?.streaming_enabled && (status === 'recording' || status === 'processing') && streamingText && (
        <div className="panel w-full max-w-lg p-0 overflow-hidden border-[var(--border-subtle)]">
          <div className="px-4 py-2 bg-[var(--bg-elevated)] border-b border-[var(--border-subtle)] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className={`led ${status === 'recording' ? 'recording' : 'processing'}`} />
              <span className="text-[0.6rem] uppercase tracking-[0.15em] text-[var(--text-muted)]">
                {status === 'recording' ? 'Transcription en direct' : 'Finalisation...'}
              </span>
            </div>
          </div>
          <div className="p-4">
            <p className="text-[var(--text-secondary)] text-sm leading-relaxed font-body italic">
              {streamingText}
              {status === 'recording' && (
                <span className="inline-block w-2 h-4 bg-[var(--accent-cyan)] ml-1 animate-pulse" />
              )}
            </p>
          </div>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="panel border-[var(--accent-red)] bg-[var(--accent-red)]/10 p-4 max-w-md">
          <div className="flex items-start gap-3">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent-red)" strokeWidth="2" className="flex-shrink-0 mt-0.5">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <div className="flex-1">
              <p className="text-[var(--text-primary)] text-sm">{error}</p>
              <button
                onClick={clearError}
                className="text-[0.65rem] uppercase tracking-wider text-[var(--accent-red)] hover:underline mt-2"
              >
                Fermer
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Result card */}
      {result && status === 'completed' && (
        <div className="result-card panel w-full max-w-lg p-0 overflow-hidden">
          {/* Header bar */}
          <div className="px-4 py-2 bg-[var(--bg-elevated)] border-b border-[var(--border-subtle)] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="led active" />
              <span className="text-[0.6rem] uppercase tracking-[0.15em] text-[var(--text-muted)]">
                Transcription
              </span>
            </div>
            <span className="text-[0.6rem] text-[var(--text-muted)] font-mono">
              {new Date().toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            </span>
          </div>

          {/* Content */}
          <div className="p-5">
            <p className="text-[var(--text-primary)] text-base leading-relaxed font-body">
              {result.text}
            </p>
          </div>

          {/* Footer stats */}
          <div className="px-4 py-3 bg-[var(--bg-elevated)] border-t border-[var(--border-subtle)] flex justify-between items-center">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-1.5">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="2">
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
                <span className="text-[0.65rem] text-[var(--text-muted)] font-mono">
                  {result.processing_time_ms}ms
                </span>
              </div>
              <div className="flex items-center gap-1.5">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-green)" strokeWidth="2">
                  <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                  <polyline points="22 4 12 14.01 9 11.01" />
                </svg>
                <span className="text-[0.65rem] text-[var(--text-muted)] font-mono">
                  {(result.confidence * 100).toFixed(0)}%
                </span>
              </div>
            </div>
            <div className="flex items-center gap-1.5">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" strokeWidth="2">
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
              </svg>
              <span className="text-[0.65rem] text-[var(--text-muted)] font-mono">
                {result.duration_seconds.toFixed(1)}s
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
