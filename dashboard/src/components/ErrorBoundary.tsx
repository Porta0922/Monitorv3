import React from 'react';
interface Props { children: React.ReactNode }
interface State { hasError: boolean }
export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { hasError: false };
  static getDerivedStateFromError() { return { hasError: true }; }
  componentDidCatch(error: Error) { console.error('Dashboard rendering failed', error); }
  render() {
    if (this.state.hasError) return <main className="empty-state" style={{ minHeight: '100dvh', justifyContent: 'center' }}><p className="eyebrow">Monitor</p><h1>No pudimos mostrar esta vista</h1><p>Puedes volver a cargarla o regresar a los equipos. Si el problema continúa, informa al administrador.</p><div className="row-actions"><button className="button" onClick={() => window.location.reload()}>Volver a cargar</button><a className="button button-primary" href="/dashboard">Ver equipos</a></div></main>;
    return this.props.children;
  }
}
