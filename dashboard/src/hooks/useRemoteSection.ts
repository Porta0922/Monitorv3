import { useCallback, useEffect, useState } from 'react';

// The caller memoizes loader by its complete query, including equipment and day.
export function useRemoteSection<T>(loader: () => Promise<T>) {
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<{loader: typeof loader; revision: number; data: T | null; error: boolean; updatedAt: Date | null} | null>(null);
  const scoped = result?.loader === loader ? result : null;
  useEffect(() => {
    let alive = true;
    loader().then(data => {
      if (alive) setResult({loader, revision, data, error: false, updatedAt: new Date()});
    }).catch(() => {
      if (alive) setResult(previous => ({loader, revision, data: previous?.loader === loader ? previous.data : null,
        updatedAt: previous?.loader === loader ? previous.updatedAt : null, error: true}));
    });
    return () => { alive = false; };
  }, [loader, revision]);
  const refresh = useCallback(() => setRevision(value => value + 1), []);
  return {data: scoped?.data ?? null, error: scoped?.error ?? false,
    loading: !scoped || scoped.revision !== revision, updatedAt: scoped?.updatedAt ?? null,
    refresh};
}
