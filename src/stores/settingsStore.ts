import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { AppSettings, AudioDevice } from '../types';

interface SettingsStore {
  settings: AppSettings | null;
  devices: AudioDevice[];
  dictionary: string[];
  isLoading: boolean;

  loadSettings: () => Promise<void>;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
  loadDevices: () => Promise<void>;
  loadDictionary: () => Promise<void>;
  addWord: (word: string) => Promise<void>;
  removeWord: (word: string) => Promise<void>;
}

const defaultSettings: AppSettings = {
  microphone_id: null,
  hotkey_push_to_talk: 'CommandOrControl+Shift+Space',
  hotkey_toggle_record: 'CommandOrControl+Shift+R',
  transcription_language: 'fr',
  auto_detect_language: false,
  theme: 'system',
  minimize_to_tray: true,
  auto_copy_to_clipboard: true,
  notification_on_complete: true,
  whisper_model: 'tiny',
  llm_enabled: false,
  llm_mode: 'basic',
  voice_commands_enabled: true,
  dictation_mode: 'general',
  streaming_enabled: true,
  auto_paste_enabled: true,
  floating_window_enabled: false,
  translation_enabled: true,
  translation_target_language: 'en',
  hotkey_translate: 'CommandOrControl+Shift+T',
};

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  devices: [],
  dictionary: [],
  isLoading: false,

  loadSettings: async () => {
    try {
      set({ isLoading: true });
      const settings = await invoke<AppSettings>('get_settings');
      set({ settings, isLoading: false });
    } catch (error) {
      console.error('Failed to load settings:', error);
      set({ settings: defaultSettings, isLoading: false });
    }
  },

  updateSettings: async (newSettings: Partial<AppSettings>) => {
    const current = get().settings || defaultSettings;
    const updated = { ...current, ...newSettings };
    try {
      await invoke('update_settings', { newSettings: updated });
      set({ settings: updated });
    } catch (error) {
      console.error('Failed to update settings:', error);
      throw error;
    }
  },

  loadDevices: async () => {
    try {
      const devices = await invoke<AudioDevice[]>('list_audio_devices');
      set({ devices });
    } catch (error) {
      console.error('Failed to load devices:', error);
    }
  },

  loadDictionary: async () => {
    try {
      const dictionary = await invoke<string[]>('get_dictionary');
      set({ dictionary });
    } catch (error) {
      console.error('Failed to load dictionary:', error);
    }
  },

  addWord: async (word: string) => {
    try {
      await invoke('add_dictionary_word', { word });
      set((state) => ({ dictionary: [...state.dictionary, word] }));
    } catch (error) {
      console.error('Failed to add word:', error);
      throw error;
    }
  },

  removeWord: async (word: string) => {
    try {
      await invoke('remove_dictionary_word', { word });
      set((state) => ({ dictionary: state.dictionary.filter((w) => w !== word) }));
    } catch (error) {
      console.error('Failed to remove word:', error);
      throw error;
    }
  },
}));
