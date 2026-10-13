export async function registerOffline(onUpdate: (worker: ServiceWorker) => void): Promise<boolean> {
  if (!('serviceWorker' in navigator)) return false;
  if (!import.meta.env.PROD) return false;
  const registration = await navigator.serviceWorker.register('/sw.js');
  if (registration.waiting) onUpdate(registration.waiting);
  registration.addEventListener('updatefound', () => {
    const worker = registration.installing;
    worker?.addEventListener('statechange', () => {
      if (worker.state === 'installed' && navigator.serviceWorker.controller) onUpdate(worker);
    });
  });
  await navigator.serviceWorker.ready;
  return true;
}
