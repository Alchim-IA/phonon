import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DictationPanel } from './components/DictationPanel';
import { TranscriptionHistory } from './components/TranscriptionHistory';
import { SettingsPanel } from './components/SettingsPanel';
import { FileTranscription } from './components/FileTranscription';
import { useSettingsStore } from './stores/settingsStore';
import { useTranscriptionStore } from './stores/transcriptionStore';
import { useHotkeys } from './hooks/useHotkeys';
import logoSvg from './assets/logo.svg';

type Tab = 'dictation' | 'history' | 'files';

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
  const { settings, loadSettings } = useSettingsStore();
  const { initialize } = useTranscriptionStore();

  useHotkeys();

  useEffect(() => {
    loadSettings();
    initialize();
  }, [loadSettings, initialize]);

  useEffect(() => {
    if (settings?.floating_window_enabled) {
      invoke('show_floating_window').catch(console.error);
    }
  }, [settings?.floating_window_enabled]);

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
              <div className="flex items-center gap-2.5 px-4 py-2 bg-[rgba(255,255,255,0.04)] border border-[var(--glass-border)] rounded-xl">
                <div className="led-frost active" />
                <span className="text-[0.7rem] text-[var(--text-secondary)] font-medium">
                  Systeme actif
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
          <div className="glass-panel px-5 py-3 flex justify-between items-center">
            <div className="flex items-center gap-4">
              <span className="tag-frost">
                {settings?.engine_type === 'whisper' && 'Whisper.cpp'}
                {settings?.engine_type === 'vosk' && 'Vosk'}
                {settings?.engine_type === 'parakeet' && 'Parakeet'}
              </span>
              {settings?.dictation_mode && settings.dictation_mode !== 'general' && (
                <span className="tag-frost accent">
                  {settings.dictation_mode === 'email' ? 'Email' : settings.dictation_mode === 'code' ? 'Code' : 'Notes'}
                </span>
              )}
              {settings?.llm_enabled && (
                <span className="tag-frost success">LLM</span>
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
