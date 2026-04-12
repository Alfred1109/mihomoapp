import { useState, useCallback } from 'react';

export type NotificationSeverity = 'success' | 'error' | 'info' | 'warning';

interface Notification {
  open: boolean;
  message: string;
  severity: NotificationSeverity;
}

interface UseNotificationResult {
  notification: Notification;
  showNotification: (message: string, severity?: NotificationSeverity) => void;
  hideNotification: () => void;
}

export function useNotification(): UseNotificationResult {
  const [notification, setNotification] = useState<Notification>({
    open: false,
    message: '',
    severity: 'info',
  });

  const showNotification = useCallback((message: string, severity: NotificationSeverity = 'info') => {
    setNotification({
      open: true,
      message,
      severity,
    });
  }, []);

  const hideNotification = useCallback(() => {
    setNotification(prev => ({ ...prev, open: false }));
  }, []);

  return { notification, showNotification, hideNotification };
}

export default useNotification;
