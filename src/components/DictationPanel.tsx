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
        return 'Capture en cours';
      case 'processing':
        return 'Analyse en cours';
      case 'completed':
        return 'Transcription terminee';
      case 'error':
        return 'Erreur systeme';
      default:
        return 'Pret a capturer';
    }
  };

  return (
    <div className="h-full flex flex-col items-center justify-center p-8 gap-10">
      {/* Main record button - Frosted orb */}
      <div className="relative animate-fade-in-up">
        {/* Outer animated rings for recording */}
        {status === 'recording' && (
          <>
            <div
              className="absolute inset-0 rounded-full border-2 border-[var(--accent-danger)]"
              style={{
                transform: 'scale(1.35)',
                animation: 'ring-expand-frost 1.5s ease-out infinite'
              }}
            />
            <div
              className="absolute inset-0 rounded-full border border-[var(--accent-danger)] opacity-50"
              style={{
                transform: 'scale(1.5)',
                animation: 'ring-expand-frost 1.5s ease-out infinite 0.5s'
              }}
            />
          </>
        )}

        {/* Outer animated rings for processing */}
        {status === 'processing' && (
          <div
            className="absolute inset-0 rounded-full"
            style={{
              transform: 'scale(1.4)',
              border: '2px dashed var(--accent-secondary)',
              animation: 'spin 8s linear infinite'
            }}
          />
        )}

        <button
          onClick={handleToggle}
          disabled={status === 'processing'}
          className={`record-orb ${status === 'recording' ? 'recording' : ''} ${status === 'processing' ? 'processing' : ''} ${status === 'processing' ? 'cursor-not-allowed' : ''}`}
          aria-label={status === 'recording' ? 'Arreter' : 'Demarrer'}
        >
          <div className="relative z-10">
            {status === 'processing' ? (
              <div className="w-12 h-12 rounded-full border-3 border-t-[var(--accent-primary)] border-r-[var(--accent-secondary)] border-b-transparent border-l-transparent animate-spin" />
            ) : status === 'recording' ? (
              <div className="w-10 h-10 bg-[var(--accent-danger)] rounded-xl shadow-lg" style={{ boxShadow: 'var(--glow-danger)' }} />
            ) : (
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="url(#mic-gradient)" strokeWidth="1.5" className="drop-shadow-lg">
                <defs>
                  <linearGradient id="mic-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="var(--accent-primary)" />
                    <stop offset="100%" stopColor="var(--accent-secondary)" />
                  </linearGradient>
                </defs>
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                <line x1="12" x2="12" y1="19" y2="22" />
              </svg>
            )}
          </div>
        </button>
      </div>

      {/* Status display */}
      <div className="text-center space-y-4 animate-fade-in-up" style={{ animationDelay: '0.1s' }}>
        <div className="flex items-center justify-center gap-3">
          <div className={`led-frost ${status === 'recording' ? 'recording' : status === 'processing' ? 'processing' : 'active'}`} />
          <span className="text-[0.875rem] text-[var(--text-secondary)] font-medium tracking-wide">
            {getStatusText()}
          </span>
          {settings?.llm_enabled && (
            <span className="tag-frost accent text-[0.6rem]">LLM</span>
          )}
        </div>

        {/* Waveform visualization */}
        {status === 'recording' && (
          <div className="space-y-4">
            <div className="waveform-frost">
              {[...Array(24)].map((_, i) => (
                <div
                  key={i}
                  className="bar"
                  style={{
                    animationDelay: `${i * 40}ms`,
                  }}
                />
              ))}
            </div>
            <div className="text-sm text-[var(--text-muted)] font-medium tabular-nums">
              {recordingDuration.toFixed(1)}s
            </div>
          </div>
        )}

        {/* Processing indicator */}
        {status === 'processing' && (
          <div className="flex flex-col items-center gap-4">
            <div className="flex items-center justify-center gap-2">
              {[...Array(5)].map((_, i) => (
                <div
                  key={i}
                  className="w-2.5 h-2.5 rounded-full bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)] bounce-dot"
                  style={{ animationDelay: `${i * 120}ms` }}
                />
              ))}
            </div>
            <span className="text-[0.8rem] text-[var(--text-muted)]">
              Transcription en cours...
            </span>
          </div>
        )}
      </div>

      {/* Streaming text display */}
      {settings?.streaming_enabled && (status === 'recording' || status === 'processing') && streamingText && (
        <div className="result-card-frost w-full max-w-lg animate-fade-in-up" style={{ animationDelay: '0.2s' }}>
          <div className="card-header">
            <div className="flex items-center gap-2.5">
              <div className={`led-frost ${status === 'recording' ? 'recording' : 'processing'}`} />
              <span className="text-[0.75rem] text-[var(--text-muted)] font-medium">
                {status === 'recording' ? 'Transcription en direct' : 'Finalisation...'}
              </span>
            </div>
          </div>
          <div className="card-content">
            <p className="text-[var(--text-secondary)] text-[0.9375rem] leading-relaxed italic">
              {streamingText}
              {status === 'recording' && (
                <span className="inline-block w-0.5 h-5 bg-gradient-to-b from-[var(--accent-primary)] to-[var(--accent-secondary)] ml-1 pulse-frost" />
              )}
            </p>
          </div>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="result-card-frost w-full max-w-md border-[var(--accent-danger)] animate-fade-in-up">
          <div className="card-content">
            <div className="flex items-start gap-4">
              <div className="w-10 h-10 rounded-xl bg-[var(--accent-danger-soft)] flex items-center justify-center flex-shrink-0">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-danger)" strokeWidth="2">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="12" y1="8" x2="12" y2="12" />
                  <line x1="12" y1="16" x2="12.01" y2="16" />
                </svg>
              </div>
              <div className="flex-1">
                <p className="text-[var(--text-primary)] text-[0.9375rem] mb-3">{error}</p>
                <button
                  onClick={clearError}
                  className="text-[0.8rem] text-[var(--accent-danger)] hover:underline font-medium"
                >
                  Fermer
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Result card */}
      {result && status === 'completed' && (
        <div className="result-card-frost w-full max-w-lg animate-fade-in-up">
          <div className="card-header">
            <div className="flex items-center gap-2.5">
              <div className="led-frost active" />
              <span className="text-[0.75rem] text-[var(--text-muted)] font-medium">
                Transcription
              </span>
            </div>
            <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
              {new Date().toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
            </span>
          </div>

          <div className="card-content">
            <p className="text-[var(--text-primary)] text-base leading-relaxed">
              {result.text}
            </p>
          </div>

          <div className="card-footer flex justify-between items-center">
            <div className="flex items-center gap-5">
              <div className="flex items-center gap-2">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="2">
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
                <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                  {result.processing_time_ms}ms
                </span>
              </div>
              <div className="flex items-center gap-2">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="2">
                  <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                  <polyline points="22 4 12 14.01 9 11.01" />
                </svg>
                <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                  {(result.confidence * 100).toFixed(0)}%
                </span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" strokeWidth="2">
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
              </svg>
              <span className="text-[0.75rem] text-[var(--text-muted)] tabular-nums">
                {result.duration_seconds.toFixed(1)}s
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
