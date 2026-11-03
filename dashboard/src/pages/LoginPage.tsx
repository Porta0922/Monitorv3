import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import { ThemeSelect } from '../components/ThemeSelect';
import { Icon } from '../components/Icon';
export function LoginPage() {
  useEffect(() => { document.title = 'Iniciar sesión · Monitor'; }, []);
  const navigate = useNavigate();
  const { login } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [otp,setOtp]=useState('');
  const [visible, setVisible] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  return <main className="login-layout"><section className="login-story"><div className="brand"><span className="brand-mark"><Icon name="activity" /></span><span>Monitor</span></div><div className="login-message"><p className="eyebrow">Visibilidad para actuar</p><h1>Cada equipo.<br />Una visión clara.</h1><p>Supervisa tu infraestructura, identifica incidentes y consulta los recursos de tus equipos desde un mismo lugar.</p><div className="login-features"><span><Icon name="monitor" />Equipos de oficina</span><span><Icon name="chart" />Históricos útiles</span><span><Icon name="shield" />Seguimiento de alertas</span></div></div><p className="login-footnote">Consola de administración de infraestructura</p></section>
    <section className="login-form-area"><div className="login-theme"><ThemeSelect /></div><form className="login-form" onSubmit={async event => { event.preventDefault(); if (busy) return; setError(''); setBusy(true); try { if (await login(username.trim(), password,otp)) navigate('/dashboard', { replace: true }); else setError('No se pudo iniciar sesión. Comprueba usuario y contraseña; si tu cuenta tiene MFA, revisa también el código. Si hubo varios intentos, espera antes de reintentar.'); } finally { setBusy(false); } }}><p className="eyebrow">Acceso a la consola</p><h2>Bienvenido de nuevo</h2><p className="muted">Ingresa con tu cuenta de Monitor.</p><label htmlFor="username">Usuario</label><input id="username" autoComplete="username" autoFocus required value={username} onChange={event => setUsername(event.target.value)} placeholder="Tu nombre de usuario" /><label htmlFor="password">Contraseña</label><div className="password-field"><input id="password" type={visible ? 'text' : 'password'} autoComplete="current-password" required value={password} onChange={event => setPassword(event.target.value)} /><button type="button" className="text-button" aria-pressed={visible} onClick={() => setVisible(v => !v)}>{visible ? 'Ocultar' : 'Mostrar'}</button></div><label htmlFor="otp">Código de autenticación (solo con MFA)</label><input id="otp" aria-describedby="otp-help" autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]{6}" maxLength={6} value={otp} onChange={event=>setOtp(event.target.value.replace(/\D/g,''))} placeholder="Código de 6 dígitos" /><p id="otp-help" className="field-note">Si configuraste un autenticador para tu cuenta, ingresa su código de 6 dígitos. Si tu cuenta no tiene MFA, deja este campo vacío.</p>{error && <div className="notice notice-error" role="alert">{error}</div>}<button className="button button-primary login-submit" disabled={busy}>{busy ? 'Iniciando sesión…' : 'Iniciar sesión'}<Icon name="arrow" /></button><p className="field-note">Si necesitas acceso, contacta al administrador de tu organización.</p></form></section>
  </main>;
}
