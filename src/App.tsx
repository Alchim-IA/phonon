import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DictationPanel } from './components/DictationPanel';
import { TranscriptionHistory } from './components/TranscriptionHistory';
import { SettingsPanel } from './components/SettingsPanel';
import { FileTranscription } from './components/FileTranscription';
import { useSettingsStore } from './stores/settingsStore';
import { useTranscriptionStore } from './stores/transcriptionStore';
import { useHotkeys } from './hooks/useHotkeys';
import { GroqQuota } from './types';
import logoSvg from './assets/logo.svg';
import { OnboardingWizard } from './components/onboarding';

type Tab = 'dictation' | 'history' | 'files';
type AppStatus = 'idle' | 'recording' | 'translating' | 'voice-action';

// Formatte un raccourci clavier pour l'affichage
function formatHotkey(hotkey: string): string {
  return hotkey
    .replace('CommandOrControl', '⌘')
    .replace('Command', '⌘')
    .replace('Control', 'Ctrl')
    .replace('Shift', '⇧')
    .replace('Alt', '⌥')
    .replace('Space', 'Espace')
    .replace(/\+/g, ' + ');
}

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dictation');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [groqQuota, setGroqQuota] = useState<GroqQuota | null>(null);
  const [appStatus, setAppStatus] = useState<AppStatus>('idle');
  const { settings, loadSettings } = useSettingsStore();
  const { initialize } = useTranscriptionStore();

  useHotkeys();

  // Écouter les événements de statut du backend
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // Recording status (PTT)
    listen<string>('recording-status', (event) => {
      if (event.payload === 'recording') {
        setAppStatus('recording');
      } else if (event.payload === 'idle') {
        setAppStatus('idle');
      }
    }).then(unlisten => unlisteners.push(unlisten));

    // Translation status
    listen<string>('translation-status', (event) => {
      if (event.payload === 'translating') {
        setAppStatus('translating');
      } else if (event.payload === 'idle') {
        setAppStatus('idle');
      }
    }).then(unlisten => unlisteners.push(unlisten));

    // Voice action status
    listen<string>('voice-action-status', (event) => {
      if (event.payload === 'recording' || event.payload === 'processing') {
        setAppStatus('voice-action');
      } else if (event.payload === 'idle') {
        setAppStatus('idle');
      }
    }).then(unlisten => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach(unlisten => unlisten());
    };
  }, []);

  const fetchGroqQuota = useCallback(async () => {
    if (settings?.llm_enabled) {
      try {
        const quota = await invoke<GroqQuota | null>('get_groq_quota');
        setGroqQuota(quota);
      } catch (e) {
        console.error('Failed to fetch Groq quota:', e);
      }
    }
  }, [settings?.llm_enabled]);

  useEffect(() => {
    loadSettings();
    initialize();
  }, [loadSettings, initialize]);

  // Récupérer le quota Groq périodiquement si LLM est actif
  useEffect(() => {
    if (settings?.llm_enabled) {
      fetchGroqQuota();
      const interval = setInterval(fetchGroqQuota, 30000); // Toutes les 30 secondes
      return () => clearInterval(interval);
    }
  }, [settings?.llm_enabled, fetchGroqQuota]);

  useEffect(() => {
    if (settings?.floating_window_enabled) {
      invoke('show_floating_window').catch(console.error);
    }
  }, [settings?.floating_window_enabled]);

  if (!settings) {
    return (
      <div className="h-screen flex flex-col overflow-hidden relative">
        <div className="mesh-gradient-bg" />
        <div className="noise-overlay" />
      </div>
    );
  }

  if (!settings.onboarding_completed) {
    return <OnboardingWizard />;
  }

  return (
    <div className="h-screen flex flex-col overflow-hidden relative">
      {/* Animated mesh gradient background */}
      <div className="mesh-gradient-bg" />

      {/* Noise texture overlay */}
      <div className="noise-overlay" />

      {/* Main content wrapper */}
      <div className="relative z-10 h-full flex flex-col">
        {/* Header */}
        <header className="flex-shrink-0 px-6 py-4">
          <div className="glass-panel px-5 py-4 flex justify-between items-center">
            <div className="flex items-center gap-5">
              {/* Logo/Title */}
              <div className="flex items-center gap-3">
                <div className="relative">
                  <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)] flex items-center justify-center shadow-lg overflow-visible">
                    <img src={logoSvg} alt="WakaScribe" className="w-64 h-64 invert" />
                  </div>
                </div>
                <div>
                  <h1 className="font-display text-lg tracking-tight">
                    <span className="text-[var(--text-primary)]">Waka</span>
                    <span className="bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] bg-clip-text text-transparent">Scribe</span>
                  </h1>
                  <p className="text-[0.65rem] text-[var(--text-muted)] tracking-wide">Dictee vocale intelligente</p>
                </div>
              </div>

              {/* Status indicator */}
              <div className={`flex items-center gap-2.5 px-4 py-2 border rounded-xl transition-all ${
                appStatus === 'idle'
                  ? 'bg-[rgba(255,255,255,0.04)] border-[var(--glass-border)]'
                  : appStatus === 'recording'
                  ? 'bg-[rgba(255,59,48,0.15)] border-[rgba(255,59,48,0.5)]'
                  : appStatus === 'translating'
                  ? 'bg-[rgba(0,122,255,0.15)] border-[rgba(0,122,255,0.5)]'
                  : 'bg-[rgba(255,179,0,0.15)] border-[rgba(255,179,0,0.5)]'
              }`}>
                <div className={`w-2.5 h-2.5 rounded-full ${
                  appStatus === 'idle'
                    ? 'bg-[var(--accent-success)]'
                    : appStatus === 'recording'
                    ? 'bg-[#FF3B30] animate-pulse'
                    : appStatus === 'translating'
                    ? 'bg-[#007AFF] animate-pulse'
                    : 'bg-[#FFB300] animate-pulse'
                }`} />
                <span className={`text-[0.7rem] font-medium ${
                  appStatus === 'idle'
                    ? 'text-[var(--text-secondary)]'
                    : appStatus === 'recording'
                    ? 'text-[#FF3B30]'
                    : appStatus === 'translating'
                    ? 'text-[#007AFF]'
                    : 'text-[#FFB300]'
                }`}>
                  {appStatus === 'idle' && 'Systeme actif'}
                  {appStatus === 'recording' && 'Transcription...'}
                  {appStatus === 'translating' && 'Traduction...'}
                  {appStatus === 'voice-action' && 'Voice Action...'}
                </span>
              </div>
            </div>

            {/* Settings button */}
            <button
              onClick={() => setSettingsOpen(true)}
              className="btn-glass"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
              <span className="hidden sm:inline">Parametres</span>
            </button>
          </div>
        </header>

        {/* Navigation tabs */}
        <nav className="flex-shrink-0 px-6">
          <div className="glass-panel overflow-hidden p-1">
            <div className="flex">
              <button
                onClick={() => setActiveTab('dictation')}
                className={`tab-frost flex-1 ${activeTab === 'dictation' ? 'active' : ''}`}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                  <line x1="12" x2="12" y1="19" y2="22" />
                </svg>
                Dictee
              </button>
              <button
                onClick={() => setActiveTab('history')}
                className={`tab-frost flex-1 ${activeTab === 'history' ? 'active' : ''}`}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
                Historique
              </button>
              <button
                onClick={() => setActiveTab('files')}
                className={`tab-frost flex-1 ${activeTab === 'files' ? 'active' : ''}`}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <path d="M9 18V5l12-2v13" />
                  <circle cx="6" cy="18" r="3" />
                  <circle cx="18" cy="16" r="3" />
                </svg>
                Fichiers
              </button>
            </div>
          </div>
        </nav>

        {/* Main content */}
        <main className="flex-1 overflow-hidden px-6 py-4">
          <div className="glass-panel h-full overflow-hidden">
            {activeTab === 'dictation' && <DictationPanel />}
            {activeTab === 'history' && <TranscriptionHistory />}
            {activeTab === 'files' && <FileTranscription isOpen={true} onClose={() => setActiveTab('dictation')} />}
          </div>
        </main>

        {/* Footer status bar */}
        <footer className="flex-shrink-0 px-6 pb-4">
          <div className="glass-panel px-5 py-3 flex justify-between items-center overflow-visible">
            <div className="flex items-center gap-4">
              <span className="tag-frost">
                {settings?.engine_type === 'whisper' && `Whisper ${settings.whisper_model.charAt(0).toUpperCase() + settings.whisper_model.slice(1)}`}
                {settings?.engine_type === 'vosk' && `Vosk ${settings.vosk_language?.toUpperCase() || ''}`}
                {settings?.engine_type === 'parakeet' && 'Parakeet TDT'}
              </span>
              {settings?.dictation_mode && settings.dictation_mode !== 'general' && (
                <span className="tag-frost accent">
                  {settings.dictation_mode === 'email' ? 'Email' : settings.dictation_mode === 'code' ? 'Code' : 'Notes'}
                </span>
              )}
              {settings?.llm_enabled && settings?.llm_provider === 'groq' && (
                <div className="relative group">
                  <span className="tag-frost success flex items-center gap-2 cursor-help">
                    LLM (Groq)
                    {groqQuota?.remaining_requests != null && groqQuota?.limit_requests != null && (
                      <span className="text-[0.6rem] opacity-80">
                        {groqQuota.remaining_requests}/{groqQuota.limit_requests}
                      </span>
                    )}
                  </span>
                  {/* Tooltip détaillé des quotas */}
                  {groqQuota && (
                    <div className="absolute bottom-full left-0 mb-2 p-3 min-w-[220px] bg-[var(--glass-bg)] backdrop-blur-xl border border-[var(--glass-border)] rounded-xl shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50">
                      <div className="text-[0.7rem] font-medium text-[var(--text-primary)] mb-2">Quotas Groq API</div>
                      <div className="space-y-2 text-[0.65rem]">
                        {/* Requêtes par jour */}
                        <div>
                          <div className="flex justify-between text-[var(--text-muted)] mb-1">
                            <span>Requetes/jour</span>
                            <span>{groqQuota.remaining_requests ?? '?'}/{groqQuota.limit_requests ?? '?'}</span>
                          </div>
                          <div className="h-1.5 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden">
                            <div
                              className="h-full bg-gradient-to-r from-[var(--accent-success)] to-[var(--accent-primary)]"
                              style={{ width: `${groqQuota.limit_requests ? ((groqQuota.remaining_requests ?? 0) / groqQuota.limit_requests * 100) : 0}%` }}
                            />
                          </div>
                          {groqQuota.reset_requests && (
                            <div className="text-[var(--text-muted)] mt-0.5">Reset: {groqQuota.reset_requests}</div>
                          )}
                        </div>
                        {/* Tokens par minute */}
                        <div>
                          <div className="flex justify-between text-[var(--text-muted)] mb-1">
                            <span>Tokens/min</span>
                            <span>{groqQuota.remaining_tokens ?? '?'}/{groqQuota.limit_tokens ?? '?'}</span>
                          </div>
                          <div className="h-1.5 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden">
                            <div
                              className="h-full bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)]"
                              style={{ width: `${groqQuota.limit_tokens ? ((groqQuota.remaining_tokens ?? 0) / groqQuota.limit_tokens * 100) : 0}%` }}
                            />
                          </div>
                          {groqQuota.reset_tokens && (
                            <div className="text-[var(--text-muted)] mt-0.5">Reset: {groqQuota.reset_tokens}</div>
                          )}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
              {settings?.llm_enabled && settings?.llm_provider === 'local' && (
                <span className="tag-frost success flex items-center gap-2">
                  LLM (Local)
                </span>
              )}
            </div>
            <div className="flex items-center gap-3">
              <span className="kbd-frost">{formatHotkey(settings?.hotkey_push_to_talk || 'CommandOrControl+Shift+Space')}</span>
              <span className="text-[0.75rem] text-[var(--text-muted)]">Push-to-talk</span>
            </div>
          </div>
        </footer>
      </div>

      {/* Settings Panel */}
      <SettingsPanel isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}

export default App;
