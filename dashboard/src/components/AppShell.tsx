import type { ReactNode } from 'react';
import { Sidebar } from './Sidebar';

interface AppShellProps {
  currentPage: string;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
  noScroll?: boolean;
}

export function AppShell({ currentPage, title, subtitle, actions, children, noScroll = false }: AppShellProps) {
  const now = new Date().toLocaleTimeString('es-AR');

  return (
    <div className="min-h-screen bg-[#070b1d] text-[#e4e6eb]">
      <div className="flex min-h-screen">
        <Sidebar currentPage={currentPage} />

        <main className={`flex-1 ${noScroll ? 'overflow-hidden' : 'overflow-y-auto'}`}>
          <div className="mx-auto w-full max-w-[1680px] px-6 py-5">
            <header className="mb-6 border-b border-[#16213f] pb-4">
              <div className="mb-3 flex items-center justify-between">
                <p className="font-display text-sm uppercase tracking-[0.18em] text-[#00d9ff]">Activity Monitor Enterprise (AME)</p>
                <p className="font-mono text-xs text-[#637197]">{now}</p>
              </div>

              <div className="flex items-end justify-between gap-4">
              <div>
                <h1 className="font-display text-3xl font-black tracking-wide text-[#e4e6eb]">{title}</h1>
                {subtitle && <p className="mt-1 text-sm text-[#8a97ba]">{subtitle}</p>}
              </div>
              {actions && <div className="flex items-center gap-3">{actions}</div>}
              </div>
            </header>

            <div className="space-y-4">{children}</div>
          </div>
        </main>
      </div>
    </div>
  );
}
