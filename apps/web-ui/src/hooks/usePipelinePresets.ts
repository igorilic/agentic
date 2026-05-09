import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type PipelinePreset = {
  id: string;
  name: string;
  agents: string[];
  createdAt: number;
  updatedAt: number;
};

export type UsePipelinePresetsResult = {
  presets: PipelinePreset[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  save: (name: string, agents: string[]) => Promise<PipelinePreset>;
  update: (id: string, name: string, agents: string[]) => Promise<PipelinePreset>;
  remove: (id: string) => Promise<void>;
};

/** Wire shape coming from Rust (snake_case fields). */
type WirePreset = {
  id: string;
  name: string;
  agents: string[];
  created_at: number;
  updated_at: number;
};

function fromWire(w: WirePreset): PipelinePreset {
  return {
    id: w.id,
    name: w.name,
    agents: w.agents,
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

export function usePipelinePresets(): UsePipelinePresetsResult {
  const [presets, setPresets] = useState<PipelinePreset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = (await invoke("list_pipeline_presets")) as WirePreset[];
      setPresets(rows.map(fromWire));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const save = useCallback(
    async (name: string, agents: string[]): Promise<PipelinePreset> => {
      const wire = (await invoke("save_pipeline_preset", { name, agents })) as WirePreset;
      await refresh();
      return fromWire(wire);
    },
    [refresh],
  );

  const update = useCallback(
    async (id: string, name: string, agents: string[]): Promise<PipelinePreset> => {
      const wire = (await invoke("update_pipeline_preset", { id, name, agents })) as WirePreset;
      await refresh();
      return fromWire(wire);
    },
    [refresh],
  );

  const remove = useCallback(
    async (id: string): Promise<void> => {
      await invoke("delete_pipeline_preset", { id });
      await refresh();
    },
    [refresh],
  );

  return { presets, loading, error, refresh, save, update, remove };
}
