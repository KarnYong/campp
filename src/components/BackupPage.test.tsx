import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { showConfirm } from './Dialog';
import { BackupPage } from './BackupPage';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('./Dialog', () => ({ showConfirm: vi.fn(), showAlert: vi.fn() }));

const backups = [
  { name: 'campp-backup-200.zip', size: 12 * 1024 * 1024, created_unix: 200 },
  { name: 'campp-backup-100-pre-restore.zip', size: 3 * 1024 * 1024, created_unix: 100 },
];

function mockBackend(overrides: Record<string, unknown> = {}) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd in overrides) {
      const v = overrides[cmd];
      if (v instanceof Error) throw v.message;
      return v;
    }
    if (cmd === 'list_backups') return backups;
    return null;
  });
}

describe('BackupPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockBackend();
  });

  it('lists backups with size and marks automatic ones', async () => {
    render(<BackupPage servicesActive={false} />);
    expect(await screen.findByText(/campp-backup-200\.zip · 12 MB/)).toBeInTheDocument();
    expect(screen.getByText(/before restore/)).toBeInTheDocument();
  });

  it('disables back up and restore while services run, but still allows delete', async () => {
    render(<BackupPage servicesActive={true} />);
    await screen.findByText(/campp-backup-200\.zip/);
    expect(screen.getByText(/Stop all services to back up or restore/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Back up now' })).toBeDisabled();
    screen.getAllByRole('button', { name: 'Restore' }).forEach((b) => expect(b).toBeDisabled());
    screen.getAllByRole('button', { name: 'Delete' }).forEach((b) => expect(b).toBeEnabled());
  });

  it('creates a backup and refreshes the list', async () => {
    mockBackend({ create_backup: 'campp-backup-300.zip' });
    render(<BackupPage servicesActive={false} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Back up now' }));
    expect(await screen.findByText('Backup saved: campp-backup-300.zip')).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([c]) => c === 'list_backups').length).toBeGreaterThan(1);
  });

  it('restores only after confirmation', async () => {
    mockBackend({ restore_backup: 'campp-backup-400-pre-restore.zip' });
    vi.mocked(showConfirm).mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    render(<BackupPage servicesActive={false} />);
    const restore = (await screen.findAllByRole('button', { name: 'Restore' }))[0];

    fireEvent.click(restore); // cancelled
    await waitFor(() => expect(showConfirm).toHaveBeenCalledTimes(1));
    expect(invoke).not.toHaveBeenCalledWith('restore_backup', expect.anything());

    fireEvent.click(restore); // confirmed
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('restore_backup', { name: 'campp-backup-200.zip' }));
    expect(await screen.findByText(/previous state was saved as campp-backup-400-pre-restore\.zip/)).toBeInTheDocument();
  });

  it('deletes a backup after confirmation', async () => {
    vi.mocked(showConfirm).mockResolvedValue(true);
    render(<BackupPage servicesActive={false} />);
    fireEvent.click((await screen.findAllByRole('button', { name: 'Delete' }))[1]);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('delete_backup', { name: 'campp-backup-100-pre-restore.zip' })
    );
  });

  it('shows the backend error when a restore is refused', async () => {
    mockBackend({ restore_backup: new Error('This backup was made on Windows.') });
    vi.mocked(showConfirm).mockResolvedValue(true);
    render(<BackupPage servicesActive={false} />);
    fireEvent.click((await screen.findAllByRole('button', { name: 'Restore' }))[0]);
    expect(await screen.findByText(/Restore failed: This backup was made on Windows\./)).toBeInTheDocument();
  });
});
