import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { FileTranscriptionResult, FileTranscriptionProgress } from '../types';

interface FileTranscriptionProps {
  isOpen: boolean;
  onClose: () => void;
}

export function FileTranscription({ isOpen }: FileTranscriptionProps) {
  const [files, setFiles] = useState<string[]>([]);
  const [results, setResults] = useState<FileTranscriptionResult[]>([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [progress, setProgress] = useState<FileTranscriptionProgress | null>(null);
  const [supportedFormats, setSupportedFormats] = useState<string[]>([]);

  useEffect(() => {
    invoke<string[]>('get_supported_audio_formats').then(setSupportedFormats).catch(console.error);
  }, []);

  useEffect(() => {
    const unlistenProgress = listen<FileTranscriptionProgress>('file-transcription-progress', (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlistenProgress.then(fn => fn());
    };
  }, []);

  const handleSelectFiles = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Audio Files',
          extensions: ['wav', 'mp3', 'm4a', 'flac', 'ogg', 'webm', 'aac', 'wma'],
        }],
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        setFiles(paths);
        setResults([]);
      }
    } catch (e) {
      console.error('Failed to open file dialog:', e);
    }
  }, []);

  const handleTranscribe = useCallback(async () => {
    if (files.length === 0) return;

    setIsProcessing(true);
    setResults([]);
    setProgress({ current: 0, total: files.length, file_name: '', status: 'starting' });

    try {
      const transcriptionResults = await invoke<FileTranscriptionResult[]>('transcribe_files', {
        paths: files,
      });
      setResults(transcriptionResults);
    } catch (e) {
      console.error('Transcription failed:', e);
    } finally {
      setIsProcessing(false);
      setProgress(null);
    }
  }, [files]);

  const handleCopyResult = useCallback((text: string) => {
    navigator.clipboard.writeText(text);
  }, []);

  const handleRemoveFile = useCallback((index: number) => {
    setFiles(prev => prev.filter((_, i) => i !== index));
  }, []);

  if (!isOpen) return null;

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6 scrollbar-thin">
        {/* File Selection */}
        <div className="space-y-4 animate-fade-in-up">
          <div className="flex items-center justify-between">
            <span className="section-title primary">
              Fichiers selectionnes ({files.length})
            </span>
            <button
              onClick={handleSelectFiles}
              disabled={isProcessing}
              className="btn-glass disabled:opacity-50"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
              Ajouter des fichiers
            </button>
          </div>

          {files.length === 0 ? (
            <div
              onClick={handleSelectFiles}
              className="glass-card p-10 text-center cursor-pointer hover:border-[var(--accent-primary)] transition-all group"
            >
              <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-[rgba(255,255,255,0.06)] border border-[var(--glass-border)] flex items-center justify-center group-hover:border-[var(--accent-primary)] transition-colors">
                <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" strokeWidth="1.5" className="group-hover:stroke-[var(--accent-primary)] transition-colors">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                  <polyline points="17 8 12 3 7 8" />
                  <line x1="12" y1="3" x2="12" y2="15" />
                </svg>
              </div>
              <p className="text-[var(--text-secondary)] text-[0.9375rem] mb-2">
                Cliquez pour selectionner des fichiers audio
              </p>
              <p className="text-[var(--text-muted)] text-[0.8rem]">
                Formats: {supportedFormats.join(', ').toUpperCase() || 'WAV, MP3, M4A, FLAC, OGG, WEBM'}
              </p>
            </div>
          ) : (
            <div className="space-y-2 max-h-48 overflow-y-auto scrollbar-thin">
              {files.map((file, index) => {
                const fileName = file.split('/').pop() || file;
                const isCurrentFile = progress?.file_name === fileName;
                return (
                  <div
                    key={index}
                    className={`glass-card p-3 flex items-center justify-between ${
                      isCurrentFile ? 'border-[var(--accent-primary)] bg-[var(--accent-primary-soft)]' : ''
                    }`}
                  >
                    <div className="flex items-center gap-3 flex-1 min-w-0">
                      <div className="w-8 h-8 rounded-lg bg-[rgba(255,255,255,0.06)] flex items-center justify-center flex-shrink-0">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" strokeWidth="1.5">
                          <path d="M9 18V5l12-2v13" />
                          <circle cx="6" cy="18" r="3" />
                          <circle cx="18" cy="16" r="3" />
                        </svg>
                      </div>
                      <span className="text-[0.875rem] text-[var(--text-primary)] truncate">{fileName}</span>
                    </div>
                    {!isProcessing && (
                      <button
                        onClick={() => handleRemoveFile(index)}
                        className="text-[var(--text-muted)] hover:text-[var(--accent-danger)] transition-colors ml-2 p-1"
                      >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <line x1="18" y1="6" x2="6" y2="18" />
                          <line x1="6" y1="6" x2="18" y2="18" />
                        </svg>
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Progress */}
        {isProcessing && progress && (
          <div className="glass-card p-5 space-y-4 animate-fade-in-up border-[var(--accent-primary)]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="led-frost processing" />
                <span className="text-[0.9375rem] text-[var(--text-primary)] font-medium">
                  {progress.status === 'transcribing' ? 'Transcription en cours...' : progress.status}
                </span>
              </div>
              <span className="tag-frost accent">
                {progress.current}/{progress.total}
              </span>
            </div>
            <div className="progress-frost">
              <div
                className="bar"
                style={{ width: `${(progress.current / progress.total) * 100}%` }}
              />
            </div>
            {progress.file_name && (
              <p className="text-[0.8rem] text-[var(--text-muted)] truncate">
                {progress.file_name}
              </p>
            )}
          </div>
        )}

        {/* Transcribe button */}
        {files.length > 0 && !isProcessing && (
          <button
            onClick={handleTranscribe}
            className="btn-primary w-full animate-fade-in-up"
          >
            <span className="flex items-center justify-center gap-2">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="5 3 19 12 5 21 5 3" />
              </svg>
              Transcrire {files.length} fichier{files.length > 1 ? 's' : ''}
            </span>
          </button>
        )}

        {/* Results */}
        {results.length > 0 && (
          <div className="space-y-4 animate-fade-in-up">
            <span className="section-title success">
              Resultats ({results.length})
            </span>
            <div className="space-y-4 stagger-children">
              {results.map((result, index) => (
                <div
                  key={index}
                  className={`result-card-frost ${
                    result.error ? 'border-[var(--accent-danger)]' : ''
                  }`}
                >
                  <div className="card-header">
                    <div className="flex items-center gap-3">
                      <div className={`w-2 h-2 rounded-full ${
                        result.error
                          ? 'bg-[var(--accent-danger)]'
                          : 'bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)]'
                      }`} />
                      <span className="text-[0.875rem] text-[var(--text-primary)] font-medium">
                        {result.file_name}
                      </span>
                    </div>
                    {result.transcription && (
                      <button
                        onClick={() => handleCopyResult(result.transcription!.text)}
                        className="btn-glass text-[0.75rem] py-1.5 px-3"
                      >
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                        </svg>
                        Copier
                      </button>
                    )}
                  </div>

                  <div className="card-content">
                    {result.error ? (
                      <p className="text-[var(--accent-danger)] text-[0.9375rem]">{result.error}</p>
                    ) : result.transcription ? (
                      <p className="text-[var(--text-secondary)] text-[0.9375rem] leading-relaxed whitespace-pre-wrap">
                        {result.transcription.text}
                      </p>
                    ) : null}
                  </div>

                  {result.transcription && (
                    <div className="card-footer">
                      <div className="flex flex-wrap gap-4 text-[0.75rem] text-[var(--text-muted)]">
                        <span className="flex items-center gap-1.5">
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <circle cx="12" cy="12" r="10" />
                            <polyline points="12 6 12 12 16 14" />
                          </svg>
                          {result.transcription.duration_seconds.toFixed(1)}s
                        </span>
                        <span className="flex items-center gap-1.5">
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                          </svg>
                          {result.transcription.processing_time_ms}ms
                        </span>
                        {result.transcription.detected_language && (
                          <span>Langue: {result.transcription.detected_language}</span>
                        )}
                        {result.transcription.model_used && (
                          <span className="tag-frost text-[0.6rem]">{result.transcription.model_used}</span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
