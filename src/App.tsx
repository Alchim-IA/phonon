import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DictationPanel } from './components/DictationPanel';
import { TranscriptionHistory } from './components/TranscriptionHistory';
import { SettingsPanel } from './components/SettingsPanel';
import { useSettingsStore } from './stores/settingsStore';
import { useTranscriptionStore } from './stores/transcriptionStore';
import { useHotkeys } from './hooks/useHotkeys';

type Tab = 'dictation' | 'history';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dictation');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { settings, loadSettings } = useSettingsStore();
  const { initialize } = useTranscriptionStore();

  useHotkeys();

  useEffect(() => {
    loadSettings();
    initialize(); // Réinitialiser l'état d'enregistrement au démarrage
  }, [loadSettings, initialize]);

  // Afficher la fenêtre flottante au démarrage si activée
  useEffect(() => {
    if (settings?.floating_window_enabled) {
      invoke('show_floating_window').catch(console.error);
    }
  }, [settings?.floating_window_enabled]);

  return (
    <div className="h-screen flex flex-col overflow-hidden relative">
      {/* Noise texture overlay */}
      <div className="noise-overlay" />

      {/* Header */}
      <header className="panel flex-shrink-0 px-5 py-4 flex justify-between items-center">
        <div className="flex items-center gap-4">
          {/* Logo/Title */}
          <div className="flex items-center gap-3">
            <div className="relative">
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[var(--accent-cyan)] to-[var(--accent-magenta)] opacity-20" />
              <div className="absolute inset-1 rounded-full bg-[var(--bg-panel)] flex items-center justify-center">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-[var(--accent-cyan)]">
                  <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                  <line x1="12" x2="12" y1="19" y2="22" />
                </svg>
              </div>
            </div>
            <h1 className="font-display font-bold text-lg tracking-tight">
              <span className="text-[var(--text-primary)]">WAKA</span>
              <span className="text-[var(--accent-cyan)] text-glow-cyan">SCRIBE</span>
            </h1>
          </div>

          {/* Status indicator */}
          <div className="flex items-center gap-2 px-3 py-1.5 bg-[var(--bg-elevated)] border border-[var(--border-subtle)]">
            <div className="led active" />
            <span className="text-[0.65rem] uppercase tracking-widest text-[var(--text-muted)]">
              Système actif
            </span>
          </div>
        </div>

        {/* Settings button */}
        <button
          onClick={() => setSettingsOpen(true)}
          className="btn-panel flex items-center gap-2"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
          <span className="hidden sm:inline">Config</span>
        </button>
      </header>

      {/* Navigation tabs */}
      <nav className="flex-shrink-0 border-b border-[var(--border-subtle)] bg-[var(--bg-panel)]">
        <div className="flex">
          <button
            onClick={() => setActiveTab('dictation')}
            className={`tab-btn flex-1 flex items-center justify-center gap-2 ${
              activeTab === 'dictation' ? 'active' : ''
            }`}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
              <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
              <line x1="12" x2="12" y1="19" y2="22" />
            </svg>
            Dictée
          </button>
          <button
            onClick={() => setActiveTab('history')}
            className={`tab-btn flex-1 flex items-center justify-center gap-2 ${
              activeTab === 'history' ? 'active' : ''
            }`}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </svg>
            Historique
          </button>
        </div>
      </nav>

      {/* Main content */}
      <main className="flex-1 overflow-hidden">
        {activeTab === 'dictation' ? <DictationPanel /> : <TranscriptionHistory />}
      </main>

      {/* Footer status bar */}
      <footer className="flex-shrink-0 px-4 py-2 bg-[var(--bg-panel)] border-t border-[var(--border-subtle)] flex justify-between items-center">
        <div className="flex items-center gap-4">
          <span className="text-[0.65rem] text-[var(--text-muted)] uppercase tracking-wider">
            Whisper.cpp
            {settings?.dictation_mode && settings.dictation_mode !== 'general' && (
              <> · {settings.dictation_mode === 'email' ? 'Email' : settings.dictation_mode === 'code' ? 'Code' : 'Notes'}</>
            )}
            {settings?.llm_enabled && <> · LLM</>}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="kbd">⌘+Shift+Space</span>
          <span className="text-[0.65rem] text-[var(--text-muted)]">Push-to-talk</span>
        </div>
      </footer>

      {/* Settings Panel */}
      <SettingsPanel isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}

export default App;
