import type { ReactNode } from 'react';
import { Sidebar } from './Sidebar';

interface AppShellProps {
  currentPage: string;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
}

export function AppShell({ currentPage, title, subtitle, actions, children }: AppShellProps) {
  return (
    <div className="min-h-screen bg-[#0a0e27] text-[#e4e6eb]">
      <div className="flex min-h-screen">
        <Sidebar currentPage={currentPage} />

        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
            <header className="mb-8 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
              <div>
                <h1 className="text-3xl font-bold tracking-tight text-[#e4e6eb] md:text-4xl">{title}</h1>
                {subtitle && <p className="mt-2 text-sm text-[#a0a5b2] md:text-base">{subtitle}</p>}
              </div>
              {actions && <div className="flex items-center gap-3">{actions}</div>}
            </header>

            <div className="space-y-6">{children}</div>
          </div>
        </main>
      </div>
    </div>
  );
}
