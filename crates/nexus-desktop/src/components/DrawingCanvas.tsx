import { useEffect, useRef, useState } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";

interface Point {
  x: number;
  y: number;
}

interface Stroke {
  points: Point[];
  color: string;
  width: number;
}

/**
 * In-chat drawing canvas with basic annotation tools.
 * Used for both one-off drawings and collaborative whiteboards.
 */
export default function DrawingCanvas({
  channelId,
  drawingId,
  isWhiteboard = false,
  onSave,
  onClose,
}: {
  channelId: string;
  drawingId?: string;
  isWhiteboard?: boolean;
  onSave?: () => void;
  onClose: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [strokes, setStrokes] = useState<Stroke[]>([]);
  const [currentStroke, setCurrentStroke] = useState<Point[]>([]);
  const [color, setColor] = useState("#ffffff");
  const [lineWidth, setLineWidth] = useState(3);
  const [drawing, setDrawing] = useState(false);
  const [saving, setSaving] = useState(false);

  const WIDTH = 800;
  const HEIGHT = 600;

  // Load existing drawing data
  useEffect(() => {
    if (!drawingId) return;
    invoke<{ drawingData: { strokes: Stroke[] } }>("get_drawing", { id: drawingId })
      .then((d) => {
        if (d.drawingData?.strokes) setStrokes(d.drawingData.strokes);
      })
      .catch(() => {});
  }, [drawingId]);

  // Redraw canvas whenever strokes change
  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#1e1e2e";
    ctx.fillRect(0, 0, WIDTH, HEIGHT);
    for (const stroke of strokes) {
      if (stroke.points.length < 2) continue;
      ctx.beginPath();
      ctx.strokeStyle = stroke.color;
      ctx.lineWidth = stroke.width;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.moveTo(stroke.points[0].x, stroke.points[0].y);
      for (let i = 1; i < stroke.points.length; i++) {
        ctx.lineTo(stroke.points[i].x, stroke.points[i].y);
      }
      ctx.stroke();
    }
    // Draw current in-progress stroke
    if (currentStroke.length > 1) {
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = lineWidth;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.moveTo(currentStroke[0].x, currentStroke[0].y);
      for (let i = 1; i < currentStroke.length; i++) {
        ctx.lineTo(currentStroke[i].x, currentStroke[i].y);
      }
      ctx.stroke();
    }
  }, [strokes, currentStroke, color, lineWidth]);

  const getPos = (e: React.MouseEvent<HTMLCanvasElement>): Point => {
    const rect = canvasRef.current!.getBoundingClientRect();
    return {
      x: ((e.clientX - rect.left) / rect.width) * WIDTH,
      y: ((e.clientY - rect.top) / rect.height) * HEIGHT,
    };
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const data = { strokes };
      if (drawingId) {
        await invoke("update_drawing", {
          id: drawingId,
          drawingData: data,
        });
      } else {
        await invoke("create_drawing", {
          channelId,
          drawingData: data,
          width: WIDTH,
          height: HEIGHT,
          isWhiteboard,
        });
      }
      onSave?.();
      onClose();
    } catch (e) {
      console.error("save drawing error", e);
    } finally {
      setSaving(false);
    }
  };

  const colors = ["#ffffff", "#ef4444", "#22c55e", "#3b82f6", "#eab308", "#a855f7"];

  return (
    <div className="flex flex-col bg-surface rounded-lg overflow-hidden border border-border">
      {/* Toolbar */}
      <div className="flex items-center gap-2 p-2 border-b border-border">
        <span className="text-sm font-medium mr-2">
          {isWhiteboard ? "Whiteboard" : "Drawing"}
        </span>
        {colors.map((c) => (
          <button
            key={c}
            className={`w-6 h-6 rounded-full border-2 ${
              color === c ? "border-white" : "border-transparent"
            }`}
            style={{ backgroundColor: c }}
            onClick={() => setColor(c)}
          />
        ))}
        <input
          type="range"
          min={1}
          max={12}
          value={lineWidth}
          onChange={(e) => setLineWidth(Number(e.target.value))}
          className="w-20 ml-2"
          title="Brush size"
        />
        <button
          onClick={() => setStrokes([])}
          className="ml-auto text-xs px-2 py-1 rounded bg-red-600/20 text-red-400 hover:bg-red-600/30"
        >
          Clear
        </button>
        <button
          onClick={() => setStrokes(strokes.slice(0, -1))}
          className="text-xs px-2 py-1 rounded bg-surface-secondary text-muted hover:bg-surface-tertiary"
        >
          Undo
        </button>
      </div>

      {/* Canvas */}
      <canvas
        ref={canvasRef}
        width={WIDTH}
        height={HEIGHT}
        className="w-full cursor-crosshair"
        style={{ aspectRatio: `${WIDTH}/${HEIGHT}` }}
        onMouseDown={(e) => {
          setDrawing(true);
          setCurrentStroke([getPos(e)]);
        }}
        onMouseMove={(e) => {
          if (!drawing) return;
          setCurrentStroke((prev) => [...prev, getPos(e)]);
        }}
        onMouseUp={() => {
          if (currentStroke.length > 1) {
            setStrokes((prev) => [
              ...prev,
              { points: currentStroke, color, width: lineWidth },
            ]);
          }
          setCurrentStroke([]);
          setDrawing(false);
        }}
        onMouseLeave={() => {
          if (currentStroke.length > 1) {
            setStrokes((prev) => [
              ...prev,
              { points: currentStroke, color, width: lineWidth },
            ]);
          }
          setCurrentStroke([]);
          setDrawing(false);
        }}
      />

      {/* Actions */}
      <div className="flex items-center justify-end gap-2 p-2 border-t border-border">
        <button
          onClick={onClose}
          className="px-3 py-1 text-sm rounded bg-surface-secondary text-muted hover:bg-surface-tertiary"
        >
          Cancel
        </button>
        <button
          onClick={handleSave}
          disabled={saving || strokes.length === 0}
          className="px-3 py-1 text-sm rounded bg-accent text-white disabled:opacity-50"
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  );
}
