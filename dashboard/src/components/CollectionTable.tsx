import { useState } from 'react';
import type { ReactNode } from 'react';
import { Icon } from './Icon';
interface Props<T> { rows: T[] | null; loading: boolean; error: boolean; title: string; note: string; searchText: (row: T) => string; columns: { label: string; render: (row: T) => ReactNode }[]; refresh: () => void }
export function CollectionTable<T>({ rows, loading, error, title, note, searchText, columns, refresh }: Props<T>) {
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(1);
  const filtered = (rows ?? []).filter(row => searchText(row).toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const pages = Math.max(1, Math.ceil(filtered.length / 25));
  const current = Math.min(page, pages);
  return <section className="panel"><div className="panel-header"><div><p className="eyebrow">Explorar registros</p><h2>{title}</h2></div><span className="muted">{error || rows === null ? 'Sin consulta disponible' : `${filtered.length} resultados`}</span></div><p className="panel-note">{note}</p><div className="fleet-toolbar"><label className="search-field"><Icon name="search" /><span className="sr-only">Buscar en {title}</span><input type="search" placeholder="Buscar en los registros consultados…" value={query} onChange={e => { setQuery(e.target.value); setPage(1); }} /></label></div>
    {loading ? <div className="empty-state" role="status">Cargando registros…</div> : error ? <div className="empty-state" role="alert"><h3>No se pudo completar la consulta</h3><p>No se puede confirmar si existen registros.</p><button className="button" onClick={refresh}>Reintentar</button></div> : filtered.length === 0 ? <div className="empty-state"><Icon name="search" /><h3>{query ? 'Sin coincidencias' : 'Todavía no hay registros'}</h3><p>{query ? 'Prueba otra búsqueda.' : 'Los datos aparecerán cuando los agentes los reporten.'}</p>{query && <button className="button" onClick={() => setQuery('')}>Limpiar búsqueda</button>}</div> : <><div className="table-scroll" tabIndex={0} role="region" aria-label={title}><table><thead><tr>{columns.map(column => <th scope="col" key={column.label}>{column.label}</th>)}</tr></thead><tbody>{filtered.slice((current-1)*25,current*25).map((row, i) => <tr key={`${current}-${i}`}>{columns.map(column => <td key={column.label}>{column.render(row)}</td>)}</tr>)}</tbody></table></div><div className="table-footer"><span>{filtered.length} registros consultados</span><div><button className="button button-small" disabled={current===1} onClick={() => setPage(current-1)}>Anterior</button><span>Página {current} de {pages}</span><button className="button button-small" disabled={current===pages} onClick={() => setPage(current+1)}>Siguiente</button></div></div></>}
  </section>;
}
