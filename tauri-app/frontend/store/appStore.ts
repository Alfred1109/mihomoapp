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
  lastConfigChange: number;
  lastProxyChange: { groupName: string; proxyName: string; timestamp: number } | null;
  lastSubscriptionUpdate: { id: string; status: string; timestamp: number } | null;
  
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
  lastConfigChange: 0,
  lastProxyChange: null,
  lastSubscriptionUpdate: null,
  
  setMihomoStatus: (status) => set({ mihomoStatus: status }),
  setIsAdmin: (isAdmin) => set({ isAdmin }),
  setAdminCheckDone: (done) => set({ adminCheckDone: done }),
  
  initEventListeners: async () => {
    get().cleanupEventListeners();
    
    const listeners: UnlistenFn[] = [];
    
    const unlistenStatus = await listen<MihomoStatusEvent>('mihomo-status', (event) => {
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
      set({ lastConfigChange: event.payload.timestamp });
    });
    listeners.push(unlistenConfig);
    
    const unlistenProxy = await listen<ProxyChangeEvent>('proxy-change', (event) => {
      set({ 
        lastProxyChange: {
          groupName: event.payload.group_name,
          proxyName: event.payload.proxy_name,
          timestamp: event.payload.timestamp,
        }
      });
    });
    listeners.push(unlistenProxy);

    const unlistenSubscription = await listen<SubscriptionUpdateEvent>('subscription-update', (event) => {
      set({
        lastSubscriptionUpdate: {
          id: event.payload.subscription_id,
          status: event.payload.status,
          timestamp: event.payload.timestamp,
        }
      });
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
