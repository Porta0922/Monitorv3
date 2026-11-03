import { useEffect, useState } from 'react';
export function useCollection<T>(loader: () => Promise<T[]>) {
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<{ loader: typeof loader; revision: number; rows: T[] | null; error: boolean } | null>(null);
  const current = result?.loader === loader && result.revision === revision ? result : null;
  useEffect(() => {
    let alive = true;
    loader().then(rows => { if (alive) setResult({ loader, revision, rows, error: false }); })
      .catch(() => { if (alive) setResult({ loader, revision, rows: null, error: true }); });
    return () => { alive = false; };
  }, [loader, revision]);
  return { rows: current?.rows ?? null, error: current?.error ?? false, loading: current === null, refresh: () => setRevision(v => v + 1) };
}
