import { create } from 'zustand';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { 
  MihomoStatus, 
  MihomoStatusEvent, 
  ConfigChangeEvent, 
  ProxyChangeEvent,
  SubscriptionUpdateEvent 
} from '../types';

interface AppStore {
  mihomoStatus: MihomoStatus;
  isAdmin: boolean;
  adminCheckDone: boolean;
  eventListeners: UnlistenFn[];
  
  setMihomoStatus: (status: MihomoStatus) => void;
  setIsAdmin: (isAdmin: boolean) => void;
  setAdminCheckDone: (done: boolean) => void;
  
  initEventListeners: () => Promise<void>;
  cleanupEventListeners: () => void;
}

export const useAppStore = create<AppStore>((set, get) => ({
  mihomoStatus: {
    running: false,
    processId: null,
    timestamp: 0,
  },
  isAdmin: false,
  adminCheckDone: false,
  eventListeners: [],
  
  setMihomoStatus: (status) => set({ mihomoStatus: status }),
  setIsAdmin: (isAdmin) => set({ isAdmin }),
  setAdminCheckDone: (done) => set({ adminCheckDone: done }),
  
  initEventListeners: async () => {
    get().cleanupEventListeners();
    
    const listeners: UnlistenFn[] = [];
    
    const unlistenStatus = await listen<MihomoStatusEvent>('mihomo-status', (event) => {
      console.log('Received mihomo-status event:', event.payload);
      set({
        mihomoStatus: {
          running: event.payload.running,
          processId: event.payload.process_id,
          timestamp: event.payload.timestamp,
        },
      });
    });
    listeners.push(unlistenStatus);
    
    const unlistenConfig = await listen<ConfigChangeEvent>('config-change', (event) => {
      console.log('Received config-change event:', event.payload);
    });
    listeners.push(unlistenConfig);
    
    const unlistenProxy = await listen<ProxyChangeEvent>('proxy-change', (event) => {
      console.log('Received proxy-change event:', event.payload);
    });
    listeners.push(unlistenProxy);

    const unlistenSubscription = await listen<SubscriptionUpdateEvent>('subscription-update', (event) => {
      console.log('Received subscription-update event:', event.payload);
    });
    listeners.push(unlistenSubscription);
    
    set({ eventListeners: listeners });
  },
  
  cleanupEventListeners: () => {
    const { eventListeners } = get();
    eventListeners.forEach(unlisten => unlisten());
    set({ eventListeners: [] });
  },
}));
