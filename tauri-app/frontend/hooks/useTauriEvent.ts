import { useEffect, useRef } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
): void {
  const handlerRef = useRef(handler);
  
  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let mounted = true;

    const setupListener = async () => {
      try {
        unlisten = await listen<T>(eventName, (event) => {
          if (mounted) {
            handlerRef.current(event.payload);
          }
        });
      } catch (error) {
        console.error(`Failed to listen to event ${eventName}:`, error);
      }
    };

    setupListener();

    return () => {
      mounted = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, [eventName]);
}

export default useTauriEvent;
