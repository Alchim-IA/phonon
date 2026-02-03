import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { TranscriptionResult, TranscriptionStatus } from '../types';

interface TranscriptionStore {
  status: TranscriptionStatus;
  result: TranscriptionResult | null;
  history: TranscriptionResult[];
  error: string | null;

  setStatus: (status: TranscriptionStatus) => void;
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<TranscriptionResult>;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  clearError: () => void;
  resetRecordingState: () => Promise<void>;
  initialize: () => Promise<void>;
}

export const useTranscriptionStore = create<TranscriptionStore>((set) => ({
  status: 'idle',
  result: null,
  history: [],
  error: null,

  setStatus: (status) => set({ status }),

  startRecording: async () => {
    try {
      set({ status: 'recording', error: null });
      await invoke('start_recording');
    } catch (error) {
      const errorStr = String(error);
      // Si l'état est bloqué, réinitialiser et réessayer une fois
      if (errorStr.includes('Already recording')) {
        console.warn('Recording state was stuck, resetting...');
        await invoke('reset_recording_state');
        set({ status: 'recording', error: null });
        await invoke('start_recording');
      } else {
        set({ status: 'error', error: errorStr });
        throw error;
      }
    }
  },

  stopRecording: async () => {
    try {
      set({ status: 'processing' });
      const result = await invoke<TranscriptionResult>('stop_recording');
      set((state) => ({
        status: 'completed',
        result,
        history: [result, ...state.history].slice(0, 50),
      }));
      return result;
    } catch (error) {
      set({ status: 'error', error: String(error) });
      throw error;
    }
  },

  loadHistory: async () => {
    try {
      const history = await invoke<TranscriptionResult[]>('get_history');
      set({ history });
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  },

  clearHistory: async () => {
    try {
      await invoke('clear_history');
      set({ history: [] });
    } catch (error) {
      console.error('Failed to clear history:', error);
    }
  },

  clearError: () => set({ error: null, status: 'idle' }),

  resetRecordingState: async () => {
    try {
      await invoke('reset_recording_state');
      set({ status: 'idle', error: null });
    } catch (error) {
      console.error('Failed to reset recording state:', error);
    }
  },

  initialize: async () => {
    try {
      // Réinitialiser l'état d'enregistrement au démarrage
      await invoke('reset_recording_state');
      set({ status: 'idle', error: null });
    } catch (error) {
      console.error('Failed to initialize transcription store:', error);
    }
  },
}));
