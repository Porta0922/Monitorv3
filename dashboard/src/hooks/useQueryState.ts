import { useSearchParams } from 'react-router-dom';
export function useQueryState(key: string, fallback = '', resetKeys: string[] = []): [string, (value: string) => void] {
  const [params, setParams] = useSearchParams();
  return [params.get(key) ?? fallback, value => setParams(previous => {
    const next = new URLSearchParams(previous);
    resetKeys.forEach(name => next.delete(name));
    if (value === fallback) next.delete(key); else next.set(key, value);
    return next;
  }, {replace: true})];
}
