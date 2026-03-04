import { useState, useEffect, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '../types';

interface StreamingChunk {
  text: string;
  is_final: boolean;
  duration_seconds: number;
}

const BAR_COUNT = 40;
const CANVAS_WIDTH = 400;
const CANVAS_HEIGHT = 60;

export function SubtitlesOverlay() {
  const [text, setText] = useState('');
  const [isVisible, setIsVisible] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [fontSize, setFontSize] = useState(20);
  const hideTimerRef = useRef<number | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const barsRef = useRef<number[]>(new Array(BAR_COUNT).fill(0));

  // Load font size from settings
  useEffect(() => {
    invoke<AppSettings>('get_settings')
      .then((settings) => {
        if (settings.subtitles_font_size) {
          setFontSize(settings.subtitles_font_size);
        }
      })
      .catch(console.error);

    const unlisten = listen<AppSettings>('settings-changed', (event) => {
      if (event.payload.subtitles_font_size) {
        setFontSize(event.payload.subtitles_font_size);
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Wave animation
  const drawWave = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const bars = barsRef.current;
    const barWidth = CANVAS_WIDTH / BAR_COUNT;
    const gap = 2;

    ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    const centerX = BAR_COUNT / 2;
    const time = Date.now() / 1000;

    for (let i = 0; i < BAR_COUNT; i++) {
      // Target height: wave pattern from center, with organic movement
      const distFromCenter = Math.abs(i - centerX) / centerX;
      const wave1 = Math.sin(time * 3 + i * 0.3) * 0.5 + 0.5;
      const wave2 = Math.sin(time * 2.3 + i * 0.5) * 0.3 + 0.5;
      const wave3 = Math.sin(time * 4.1 + i * 0.2) * 0.2 + 0.5;
      const envelope = 1 - distFromCenter * 0.6;
      const target = isRecording
        ? Math.max(0.08, (wave1 * 0.5 + wave2 * 0.3 + wave3 * 0.2) * envelope)
        : 0.05;

      // Smooth interpolation
      bars[i] += (target - bars[i]) * 0.15;

      const barHeight = Math.max(2, bars[i] * CANVAS_HEIGHT);
      const x = i * barWidth + gap / 2;
      const y = (CANVAS_HEIGHT - barHeight) / 2;

      // Gradient color: center is brighter
      const brightness = 1 - distFromCenter * 0.4;
      const alpha = isRecording ? 0.7 + brightness * 0.3 : 0.3;
      ctx.fillStyle = `rgba(255, 255, 255, ${alpha})`;
      ctx.beginPath();
      ctx.roundRect(x, y, barWidth - gap, barHeight, 1.5);
      ctx.fill();
    }

    animFrameRef.current = requestAnimationFrame(drawWave);
  }, [isRecording]);

  // Start/stop animation
  useEffect(() => {
    if (isVisible) {
      animFrameRef.current = requestAnimationFrame(drawWave);
    }
    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
      }
    };
  }, [isVisible, drawWave]);

  // Listen for events
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<StreamingChunk>('transcription-chunk', (event) => {
      const chunk = event.payload;
      if (chunk.text && chunk.text.trim()) {
        setText(chunk.text);

        if (hideTimerRef.current) {
          clearTimeout(hideTimerRef.current);
        }

        if (chunk.is_final) {
          setIsRecording(false);
          hideTimerRef.current = window.setTimeout(() => {
            setIsVisible(false);
            setText('');
          }, 3000);
        }
      }
    }).then(unlisten => unlisteners.push(unlisten));

    listen<string>('recording-status', (event) => {
      if (event.payload === 'recording') {
        setIsVisible(true);
        setIsRecording(true);
        setText('');
      } else if (event.payload === 'processing') {
        setIsRecording(false);
      } else if (event.payload === 'idle') {
        setIsRecording(false);
        if (hideTimerRef.current) {
          clearTimeout(hideTimerRef.current);
        }
        hideTimerRef.current = window.setTimeout(() => {
          setIsVisible(false);
          setText('');
        }, 3000);
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
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'move',
        userSelect: 'none',
        opacity: isVisible ? 1 : 0,
        transition: 'opacity 0.3s ease',
        pointerEvents: isVisible ? 'auto' : 'none',
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: 500,
          padding: '16px 24px',
          background: 'rgba(0, 0, 0, 0.8)',
          borderRadius: '16px',
          backdropFilter: 'blur(20px)',
          WebkitBackdropFilter: 'blur(20px)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '12px',
        }}
      >
        {/* Wave visualizer */}
        <canvas
          ref={canvasRef}
          width={CANVAS_WIDTH}
          height={CANVAS_HEIGHT}
          style={{
            width: CANVAS_WIDTH / 2,
            height: CANVAS_HEIGHT / 2,
            opacity: isRecording ? 1 : 0.4,
            transition: 'opacity 0.3s ease',
          }}
        />

        {/* Transcription text below the wave */}
        {text && (
          <p
            style={{
              color: 'rgba(255, 255, 255, 0.95)',
              fontSize: `${fontSize}px`,
              fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
              fontWeight: 500,
              margin: 0,
              lineHeight: 1.4,
              textAlign: 'center',
              maxWidth: '100%',
              wordBreak: 'break-word',
            }}
          >
            {text}
          </p>
        )}
      </div>
    </div>
  );
}
