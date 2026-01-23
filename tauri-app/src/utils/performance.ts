/**
 * 性能优化工具函数
 * 借鉴自 nyanpasu 的性能优化技术
 */

/**
 * 防抖函数 - 延迟执行，只执行最后一次
 */
export function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: number | null = null;
  
  return function executedFunction(...args: Parameters<T>) {
    const later = () => {
      timeout = null;
      func(...args);
    };
    
    if (timeout) {
      clearTimeout(timeout);
    }
    timeout = setTimeout(later, wait);
  };
}

/**
 * 节流函数 - 限制执行频率
 */
export function throttle<T extends (...args: any[]) => any>(
  func: T,
  limit: number
): (...args: Parameters<T>) => void {
  let inThrottle: boolean = false;
  
  return function executedFunction(...args: Parameters<T>) {
    if (!inThrottle) {
      func(...args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
}

/**
 * 延迟数据缓存
 */
interface CacheItem<T> {
  data: T;
  timestamp: number;
}

class DataCache<T> {
  private cache: Map<string, CacheItem<T>> = new Map();
  private ttl: number;

  constructor(ttlSeconds: number = 60) {
    this.ttl = ttlSeconds * 1000;
  }

  set(key: string, data: T): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });
  }

  get(key: string): T | null {
    const item = this.cache.get(key);
    if (!item) return null;

    const age = Date.now() - item.timestamp;
    if (age > this.ttl) {
      this.cache.delete(key);
      return null;
    }

    return item.data;
  }

  clear(): void {
    this.cache.clear();
  }

  has(key: string): boolean {
    const item = this.cache.get(key);
    if (!item) return false;

    const age = Date.now() - item.timestamp;
    if (age > this.ttl) {
      this.cache.delete(key);
      return false;
    }

    return true;
  }
}

// 导出单例缓存实例
export const proxyCache = new DataCache<any>(30); // 30秒缓存
export const configCache = new DataCache<any>(60); // 60秒缓存

/**
 * 延迟执行 - 避免阻塞UI
 */
export function defer(callback: () => void, delay: number = 0): void {
  setTimeout(callback, delay);
}

/**
 * 批量执行 - 分批处理大量数据
 */
export async function batchProcess<T, R>(
  items: T[],
  processor: (item: T) => Promise<R>,
  batchSize: number = 10
): Promise<R[]> {
  const results: R[] = [];
  
  for (let i = 0; i < items.length; i += batchSize) {
    const batch = items.slice(i, i + batchSize);
    const batchResults = await Promise.all(batch.map(processor));
    results.push(...batchResults);
    
    // 让出控制权，避免阻塞UI
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  
  return results;
}

/**
 * 懒加载 - 延迟加载非关键数据
 */
export function lazyLoad<T>(
  loader: () => Promise<T>,
  delay: number = 500
): Promise<T> {
  return new Promise((resolve, reject) => {
    setTimeout(() => {
      loader().then(resolve).catch(reject);
    }, delay);
  });
}
