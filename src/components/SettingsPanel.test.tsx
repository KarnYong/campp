import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { SettingsPanel } from './SettingsPanel';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('./Dialog', () => ({ showConfirm: vi.fn().mockResolvedValue(true), showAlert: vi.fn() }));

const settings = {
  web_port: 8080, php_port: 9000, mysql_port: 3307, postgres_port: 5433,
  project_root: '', mysql_root_password: 'secret', postgres_root_password: 'pgsecret',
  listen_all_interfaces: false, ngrok_authtoken: '', theme: 'system',
};

function mockBackend(overrides: Record<string, unknown> = {}) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd in overrides) return overrides[cmd];
    if (cmd === 'get_settings') return settings;
    if (cmd === 'check_existing_components') return { caddy: '2.11.7', php: '8.5.12', mysql: '8.4.11', postgresql: '18.6.1' };
    return null;
  });
}

describe('SettingsPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockBackend();
  });

  it('is read-only while services run', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={true} />);
    expect(await screen.findByText(/Stop all services to change settings/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    expect(screen.getByLabelText(/^Web server/)).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Delete everything…' })).toBeDisabled();
  });

  it('refuses to save an empty database password', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    const pw = await screen.findByLabelText('PostgreSQL root password');
    fireEvent.change(pw, { target: { value: '  ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(await screen.findByText("PostgreSQL root password can't be empty.")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('save_settings', expect.anything());
  });

  it('saves changed ports and passwords', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    fireEvent.change(await screen.findByLabelText(/^Web server/), { target: { value: '8081' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_settings', {
        settings: expect.objectContaining({ web_port: 8081, mysql_root_password: 'secret' }),
      })
    );
  });

  it('deletes everything only after the inline confirmation', async () => {
    mockBackend({ delete_all_data: 'campp-backup-1-pre-delete.zip' });
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Delete everything…' }));
    expect(invoke).not.toHaveBeenCalledWith('delete_all_data');
    fireEvent.click(screen.getByRole('button', { name: 'Yes, delete everything' }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('delete_all_data'));
    expect(await screen.findByText(/campp-backup-1-pre-delete\.zip/)).toBeInTheDocument();
  });
});
