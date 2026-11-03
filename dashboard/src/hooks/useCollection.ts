import { useRemoteSection } from './useRemoteSection';
export function useCollection<T>(loader: () => Promise<T[]>) {
  const {data, ...state} = useRemoteSection(loader);
  return {rows: data, ...state};
}
