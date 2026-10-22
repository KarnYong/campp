import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { ThemeSwitch } from './ThemeSwitch';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('ThemeSwitch', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete document.documentElement.dataset.theme;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => (cmd === 'get_settings' ? { theme: 'light' } : null));
  });

  it('shows the saved theme as selected', async () => {
    render(<ThemeSwitch />);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Always light' })).toHaveAttribute('aria-pressed', 'true')
    );
  });

  it('applies and saves a theme on click, without touching services', async () => {
    render(<ThemeSwitch />);
    fireEvent.click(screen.getByRole('button', { name: 'Always dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('set_theme', { theme: 'dark' }));
    expect(invoke).not.toHaveBeenCalledWith('save_settings', expect.anything());

    fireEvent.click(screen.getByRole('button', { name: 'Match your operating system' }));
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });
});
