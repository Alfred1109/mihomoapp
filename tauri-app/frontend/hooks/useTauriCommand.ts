import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface UseTauriCommandResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  execute: (...args: unknown[]) => Promise<T | null>;
  reset: () => void;
}

export function useTauriCommand<T>(command: string): UseTauriCommandResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(async (...args: unknown[]): Promise<T | null> => {
    setLoading(true);
    setError(null);
    
    try {
      const result = await invoke<T>(command, args[0] as Record<string, unknown> | undefined);
      setData(result);
      return result;
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : String(e);
      setError(errorMessage);
      return null;
    } finally {
      setLoading(false);
    }
  }, [command]);

  const reset = useCallback(() => {
    setData(null);
    setError(null);
    setLoading(false);
  }, []);

  return { data, loading, error, execute, reset };
}

export function useLazyTauriCommand<T, P = Record<string, unknown>>(
  command: string
): {
  execute: (params?: P) => Promise<T>;
  loading: boolean;
  error: string | null;
} {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(async (params?: P): Promise<T> => {
    setLoading(true);
    setError(null);
    
    try {
      const result = await invoke<T>(command, params as Record<string, unknown> | undefined);
      return result;
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : String(e);
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [command]);

  return { execute, loading, error };
}

export default useTauriCommand;
