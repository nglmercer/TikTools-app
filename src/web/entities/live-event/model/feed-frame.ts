export const scheduleFrame = (callback: () => void): number => {
  if (typeof requestAnimationFrame === 'function') return requestAnimationFrame(callback);
  callback();
  return 0;
};
export const cancelFrame = (id: number): void => {
  if (id !== 0 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(id);
};
