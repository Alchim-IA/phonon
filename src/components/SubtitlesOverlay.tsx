import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '../types';

interface StreamingChunk {
  text: string;
  is_final: boolean;
  duration_seconds: number;
}

export function SubtitlesOverlay() {
  const [text, setText] = useState('');
  const [isVisible, setIsVisible] = useState(false);
  const [fontSize, setFontSize] = useState(20);
  const hideTimerRef = useRef<number | null>(null);

  // Load font size from settings
  useEffect(() => {
    invoke<AppSettings>('get_settings')
      .then((settings) => {
        if (settings.subtitles_font_size) {
          setFontSize(settings.subtitles_font_size);
        }
      })
      .catch(console.error);

    // Listen for settings changes
    const unlisten = listen<AppSettings>('settings-changed', (event) => {
      if (event.payload.subtitles_font_size) {
        setFontSize(event.payload.subtitles_font_size);
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<StreamingChunk>('transcription-chunk', (event) => {
      const chunk = event.payload;
      if (chunk.text && chunk.text.trim()) {
        setText(chunk.text);
        setIsVisible(true);

        // Reset hide timer
        if (hideTimerRef.current) {
          clearTimeout(hideTimerRef.current);
        }

        if (chunk.is_final) {
          // Hide after 5 seconds for final text
          hideTimerRef.current = window.setTimeout(() => {
            setIsVisible(false);
            setText('');
          }, 5000);
        }
      }
    }).then(unlisten => unlisteners.push(unlisten));

    listen<string>('recording-status', (event) => {
      if (event.payload === 'recording') {
        setIsVisible(true);
        setText('');
      } else if (event.payload === 'idle') {
        // Keep showing for a bit after idle
        if (hideTimerRef.current) {
          clearTimeout(hideTimerRef.current);
        }
        hideTimerRef.current = window.setTimeout(() => {
          setIsVisible(false);
          setText('');
        }, 5000);
      }
    }).then(unlisten => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach(unlisten => unlisten());
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
    };
  }, []);

  return (
    <div
      data-tauri-drag-region
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'center',
        cursor: 'move',
        userSelect: 'none',
      }}
    >
      <div
        style={{
          width: '100%',
          padding: '12px 24px',
          background: isVisible ? 'rgba(0, 0, 0, 0.75)' : 'transparent',
          borderRadius: '12px',
          transition: 'opacity 0.3s ease, background 0.3s ease',
          opacity: isVisible && text ? 1 : 0,
          textAlign: 'center',
        }}
      >
        <p
          style={{
            color: 'white',
            fontSize: `${fontSize}px`,
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            fontWeight: 500,
            margin: 0,
            lineHeight: 1.4,
            textShadow: '0 1px 3px rgba(0,0,0,0.5)',
          }}
        >
          {text}
        </p>
      </div>
    </div>
  );
}
