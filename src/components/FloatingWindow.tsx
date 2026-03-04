import { useEffect, useState, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { StreamingChunk } from "../types";

type RecordingStatus = "idle" | "recording" | "processing";

const BAR_COUNT = 48;
const CANVAS_WIDTH = 600;
const CANVAS_HEIGHT = 80;

export default function FloatingWindow() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [streamingText, setStreamingText] = useState<string>("");
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const barsRef = useRef<number[]>(new Array(BAR_COUNT).fill(0));

  const handleMouseDown = async (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button")) return;
    const window = getCurrentWindow();
    await window.startDragging();
  };

  // Wave animation
  const isRecording = status === "recording";

  const drawWave = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const bars = barsRef.current;
    const barWidth = CANVAS_WIDTH / BAR_COUNT;
    const gap = 3;
    const centerX = BAR_COUNT / 2;
    const time = Date.now() / 1000;

    ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    for (let i = 0; i < BAR_COUNT; i++) {
      const distFromCenter = Math.abs(i - centerX) / centerX;

      // Organic multi-wave pattern
      const wave1 = Math.sin(time * 2.8 + i * 0.35) * 0.5 + 0.5;
      const wave2 = Math.sin(time * 1.9 + i * 0.55) * 0.35 + 0.5;
      const wave3 = Math.sin(time * 4.2 + i * 0.18) * 0.15 + 0.5;
      const envelope = 1 - distFromCenter * 0.7;

      const target = isRecording
        ? Math.max(0.06, (wave1 * 0.5 + wave2 * 0.3 + wave3 * 0.2) * envelope)
        : 0.03;

      // Smooth interpolation
      bars[i] += (target - bars[i]) * 0.12;

      const barHeight = Math.max(2, bars[i] * CANVAS_HEIGHT);
      const x = i * barWidth + gap / 2;
      const y = (CANVAS_HEIGHT - barHeight) / 2;

      // Gradient: cyan center, fading edges
      const brightness = 1 - distFromCenter * 0.5;
      const alpha = isRecording ? 0.5 + brightness * 0.5 : 0.15;

      // Color: from cyan to blue at edges
      const r = Math.round(0 + distFromCenter * 40);
      const g = Math.round(229 - distFromCenter * 100);
      const b = 255;
      ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${alpha})`;

      ctx.beginPath();
      ctx.roundRect(x, y, barWidth - gap, barHeight, 2);
      ctx.fill();
    }

    animFrameRef.current = requestAnimationFrame(drawWave);
  }, [isRecording]);

  // Start/stop animation
  useEffect(() => {
    if (status !== "idle") {
      animFrameRef.current = requestAnimationFrame(drawWave);
    }
    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
      }
    };
  }, [status, drawWave]);

  // Listen to events
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<string>("recording-status", (event) => {
      const newStatus = event.payload as RecordingStatus;
      setStatus(newStatus);
      if (newStatus === "recording") {
        setStreamingText("");
      }
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<StreamingChunk>("transcription-chunk", (event) => {
      setStreamingText(event.payload.text);
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  return (
    <div
      className="floating-window"
      data-tauri-drag-region
      onMouseDown={handleMouseDown}
      style={{ cursor: "grab" }}
    >
      {/* Wave visualizer */}
      <div
        data-tauri-drag-region
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          padding: "20px 16px 8px",
        }}
      >
        <canvas
          ref={canvasRef}
          width={CANVAS_WIDTH}
          height={CANVAS_HEIGHT}
          data-tauri-drag-region
          style={{
            width: CANVAS_WIDTH / 2,
            height: CANVAS_HEIGHT / 2,
            opacity: isRecording ? 1 : 0.4,
            transition: "opacity 0.3s ease",
          }}
        />
      </div>

      {/* Status indicator */}
      <div
        data-tauri-drag-region
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          gap: "8px",
          padding: "4px 0",
        }}
      >
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: isRecording ? "#ff3b3b" : status === "processing" ? "#00e5ff" : "#00ff88",
            boxShadow: isRecording
              ? "0 0 10px rgba(255, 59, 59, 0.6)"
              : status === "processing"
              ? "0 0 10px rgba(0, 229, 255, 0.5)"
              : "0 0 10px rgba(0, 255, 136, 0.4)",
            animation: isRecording ? "led-pulse-frost 1s ease-in-out infinite" : "none",
          }}
        />
        <span
          style={{
            fontSize: "0.6rem",
            fontWeight: 600,
            letterSpacing: "0.12em",
            color: "rgba(255, 255, 255, 0.5)",
            textTransform: "uppercase",
          }}
        >
          {isRecording ? "Ecoute..." : status === "processing" ? "Traitement..." : "Pret"}
        </span>
      </div>

      {/* Transcription text */}
      {streamingText && (
        <div
          data-tauri-drag-region
          style={{
            padding: "8px 20px 16px",
            textAlign: "center",
          }}
        >
          <p
            style={{
              margin: 0,
              fontSize: "0.85rem",
              lineHeight: 1.5,
              color: status === "recording" ? "rgba(255, 255, 255, 0.6)" : "rgba(255, 255, 255, 0.9)",
              fontStyle: status === "recording" ? "italic" : "normal",
              fontFamily: "'DM Sans', system-ui, sans-serif",
              fontWeight: 400,
            }}
          >
            {streamingText}
          </p>
        </div>
      )}
    </div>
  );
}
