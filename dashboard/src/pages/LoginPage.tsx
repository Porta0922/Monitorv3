import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';

export function LoginPage() {
  const navigate = useNavigate();
  const { login } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsLoading(true);

    const success = await login(username, password);
    if (success) {
      navigate('/dashboard');
    } else {
      setError('Invalid username or password');
    }

    setIsLoading(false);
  };

  return (
    <div
      className="flex min-h-screen items-center justify-center bg-[#0a0e27]"
      style={{ backgroundImage: 'radial-gradient(circle at 10% 0%, rgba(0,217,255,0.16), transparent 38%), radial-gradient(circle at 90% 100%, rgba(0,255,136,0.1), transparent 32%)' }}
    >
      <div className="cyber-card w-[520px] rounded-2xl p-8">
        <h1 className="font-display mb-2 text-center text-3xl font-black text-[#e4e6eb]">CYBERPUNK NEXUS</h1>
        <p className="mb-8 text-center text-sm text-[#a0a5b2]">Autenticacion de consola central</p>

        <form onSubmit={handleSubmit} className="space-y-5">
          <div>
            <label className="mb-2 block text-xs font-semibold uppercase tracking-wider text-[#00d9ff]">Usuario</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="admin"
              className="w-full rounded-lg border border-[#1e2339] bg-[#0a0e27] px-4 py-3 text-[#e4e6eb]"
              required
            />
          </div>

          <div>
            <label className="mb-2 block text-xs font-semibold uppercase tracking-wider text-[#00d9ff]">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              className="w-full rounded-lg border border-[#1e2339] bg-[#0a0e27] px-4 py-3 text-[#e4e6eb]"
              required
            />
          </div>

          {error && <div className="rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-3 text-sm text-red-300">{error}</div>}

          <button
            type="submit"
            disabled={isLoading}
            className="w-full rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/15 px-4 py-3 text-sm font-semibold uppercase tracking-wider text-[#00d9ff] hover:bg-[#00d9ff]/25 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {isLoading ? 'Conectando...' : 'Ingresar al Nexus'}
          </button>
        </form>
      </div>
    </div>
  );
}
