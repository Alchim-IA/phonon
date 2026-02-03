import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { DictationMode, StreamingChunk } from "../types";

type RecordingStatus = "idle" | "recording" | "processing";

interface StatusInfo {
  mode: DictationMode;
  llmEnabled: boolean;
  duration: number;
}

// Dimensions de la fenêtre
const COMPACT_SIZE = { width: 300, height: 40 };
const EXTENDED_SIZE = { width: 400, height: 150 };

export default function FloatingWindow() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [streamingText, setStreamingText] = useState<string>("");
  const [statusInfo, setStatusInfo] = useState<StatusInfo>({
    mode: "general",
    llmEnabled: false,
    duration: 0,
  });
  const [isDragging, setIsDragging] = useState(false);
  const dragOffset = useRef({ x: 0, y: 0 });
  const durationInterval = useRef<ReturnType<typeof setInterval> | null>(null);

  // Gestion du drag & drop pour déplacer la fenêtre
  const handleMouseDown = async (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button")) return;

    setIsDragging(true);
    const window = getCurrentWindow();
    const pos = await window.outerPosition();
    dragOffset.current = {
      x: e.screenX - pos.x,
      y: e.screenY - pos.y,
    };
  };

  const handleMouseMove = async (e: React.MouseEvent) => {
    if (!isDragging) return;

    const newX = e.screenX - dragOffset.current.x;
    const newY = e.screenY - dragOffset.current.y;

    await invoke("set_floating_window_position", { x: newX, y: newY });
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Redimensionner la fenêtre selon le statut
  useEffect(() => {
    const resizeWindow = async () => {
      const size = status === "idle" ? COMPACT_SIZE : EXTENDED_SIZE;
      await invoke("set_floating_window_size", {
        width: size.width,
        height: size.height
      });
    };
    resizeWindow();
  }, [status]);

  // Écouter les événements de transcription
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // Statut d'enregistrement
    listen<string>("recording-status", (event) => {
      const newStatus = event.payload as RecordingStatus;
      setStatus(newStatus);

      if (newStatus === "recording") {
        setStreamingText("");
        setStatusInfo((prev) => ({ ...prev, duration: 0 }));
        // Démarrer le compteur de durée
        durationInterval.current = setInterval(() => {
          setStatusInfo((prev) => ({ ...prev, duration: prev.duration + 0.1 }));
        }, 100);
      } else {
        // Arrêter le compteur
        if (durationInterval.current) {
          clearInterval(durationInterval.current);
          durationInterval.current = null;
        }
      }
    }).then((unlisten) => unlisteners.push(unlisten));

    // Chunks de streaming
    listen<StreamingChunk>("transcription-chunk", (event) => {
      const chunk = event.payload;
      if (chunk.is_final) {
        setStreamingText(chunk.text);
      } else {
        setStreamingText((prev) => prev + chunk.text);
      }
    }).then((unlisten) => unlisteners.push(unlisten));

    // Infos de statut (mode, LLM)
    listen<StatusInfo>("floating-status-info", (event) => {
      setStatusInfo((prev) => ({ ...prev, ...event.payload }));
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
      if (durationInterval.current) {
        clearInterval(durationInterval.current);
      }
    };
  }, []);

  // Fermer la fenêtre
  const handleClose = async () => {
    await invoke("hide_floating_window");
  };

  // Minimiser la fenêtre
  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  // Label de statut
  const getStatusLabel = () => {
    switch (status) {
      case "idle":
        return "PRET";
      case "recording":
        return "CAPTURE EN COURS";
      case "processing":
        return "TRAITEMENT...";
    }
  };

  // Mode affiché
  const getModeLabel = () => {
    const labels: Record<DictationMode, string> = {
      general: "General",
      email: "Email",
      code: "Code",
      notes: "Notes",
    };
    return labels[statusInfo.mode];
  };

  return (
    <div
      className="floating-window"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      {/* Barre de titre compacte */}
      <div className="floating-header">
        <div className="floating-status">
          <span
            className={`status-led ${status === "recording" ? "recording" : status === "processing" ? "processing" : "idle"}`}
          />
          <span className="status-label">{getStatusLabel()}</span>
        </div>
        <div className="floating-controls">
          <button onClick={handleMinimize} className="control-btn" title="Minimiser">
            -
          </button>
          <button onClick={handleClose} className="control-btn close" title="Fermer">
            x
          </button>
        </div>
      </div>

      {/* Zone de texte (visible en mode étendu) */}
      {status !== "idle" && (
        <>
          <div className="floating-content">
            <p className={`streaming-text ${status === "recording" ? "provisional" : ""}`}>
              {streamingText || "..."}
            </p>
          </div>
          <div className="floating-footer">
            <span className="mode-badge">{getModeLabel()}</span>
            {statusInfo.llmEnabled && <span className="mode-badge llm">LLM</span>}
            <span className="duration">{statusInfo.duration.toFixed(1)}s</span>
          </div>
        </>
      )}
    </div>
  );
}
