import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';

interface MihomoStatus {
  running: boolean;
  processId: number | null;
  timestamp: number;
}

interface AppStore {
  mihomoStatus: MihomoStatus;
  isAdmin: boolean;
  adminCheckDone: boolean;
  
  setMihomoStatus: (status: MihomoStatus) => void;
  setIsAdmin: (isAdmin: boolean) => void;
  setAdminCheckDone: (done: boolean) => void;
  
  initEventListeners: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set) => ({
  mihomoStatus: {
    running: false,
    processId: null,
    timestamp: 0,
  },
  isAdmin: false,
  adminCheckDone: false,
  
  setMihomoStatus: (status) => set({ mihomoStatus: status }),
  setIsAdmin: (isAdmin) => set({ isAdmin }),
  setAdminCheckDone: (done) => set({ adminCheckDone: done }),
  
  initEventListeners: async () => {
    await listen('mihomo-status', (event: any) => {
      console.log('Received mihomo-status event:', event.payload);
      set({
        mihomoStatus: {
          running: event.payload.running,
          processId: event.payload.process_id,
          timestamp: event.payload.timestamp,
        },
      });
    });
    
    await listen('config-change', (event: any) => {
      console.log('Received config-change event:', event.payload);
    });
    
    await listen('proxy-change', (event: any) => {
      console.log('Received proxy-change event:', event.payload);
    });
  },
}));
