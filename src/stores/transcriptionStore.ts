import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { TranscriptionResult, TranscriptionStatus } from '../types';

interface TranscriptionStore {
  status: TranscriptionStatus;
  result: TranscriptionResult | null;
  history: TranscriptionResult[];
  error: string | null;

  startRecording: () => Promise<void>;
  stopRecording: () => Promise<TranscriptionResult>;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  clearError: () => void;
}

export const useTranscriptionStore = create<TranscriptionStore>((set) => ({
  status: 'idle',
  result: null,
  history: [],
  error: null,

  startRecording: async () => {
    try {
      set({ status: 'recording', error: null });
      await invoke('start_recording');
    } catch (error) {
      set({ status: 'error', error: String(error) });
      throw error;
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
}));
