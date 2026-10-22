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

function mockBackend(overrides: Record<string, unknown> = {}, s: typeof settings = settings) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd in overrides) return overrides[cmd];
    if (cmd === 'get_settings') return s;
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

  it('never shows the current passwords', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    expect(await screen.findByLabelText('New PostgreSQL root password')).toHaveValue('');
    expect(screen.getByLabelText(/^New .* root password$/, { selector: '#mysql-password' })).toHaveValue('');
    expect(screen.queryByDisplayValue('secret')).not.toBeInTheDocument();
    expect(screen.queryByDisplayValue('pgsecret')).not.toBeInTheDocument();
  });

  it('keeps the current password when the field is left empty', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    await screen.findByLabelText('New PostgreSQL root password');
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_settings', {
        settings: expect.objectContaining({ mysql_root_password: 'secret', postgres_root_password: 'pgsecret' }),
      })
    );
  });

  it('saves a new password and clears the field', async () => {
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    const pw = await screen.findByLabelText('New PostgreSQL root password');
    fireEvent.change(pw, { target: { value: 'Brand-new-pw-42' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_settings', {
        settings: expect.objectContaining({ postgres_root_password: 'Brand-new-pw-42', mysql_root_password: 'secret' }),
      })
    );
    await waitFor(() => expect(pw).toHaveValue(''));
  });

  it('refuses to save when a database would have no password at all', async () => {
    mockBackend({}, { ...settings, postgres_root_password: '' });
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    fireEvent.change(await screen.findByLabelText('New PostgreSQL root password'), { target: { value: '   ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(await screen.findByText("PostgreSQL root password can't be empty.")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('save_settings', expect.anything());
  });

  it('warns about weak database passwords only in LAN mode', async () => {
    // 'secret' (MySQL) is shorter than 8 characters
    mockBackend({}, { ...settings, listen_all_interfaces: true });
    const { unmount } = render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    expect(await screen.findByText(/database root password is weak/)).toBeInTheDocument();

    // Typing a strong new MySQL password clears it (PostgreSQL's 'pgsecret' is 8 chars, not common)
    fireEvent.change(screen.getByLabelText(/root password$/, { selector: '#mysql-password' }), {
      target: { value: 'Long-and-unique-77' },
    });
    expect(screen.queryByText(/database root password is weak/)).not.toBeInTheDocument();
    unmount();

    mockBackend(); // Local only: no warning even with the weak password
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    await screen.findByLabelText('New PostgreSQL root password');
    expect(screen.queryByText(/database root password is weak/)).not.toBeInTheDocument();
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
    mockBackend({ delete_all_data: null });
    render(<SettingsPanel onClose={() => {}} servicesActive={false} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Delete everything…' }));
    expect(invoke).not.toHaveBeenCalledWith('delete_all_data');
    fireEvent.click(screen.getByRole('button', { name: 'Yes, delete everything' }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('delete_all_data'));
    expect(await screen.findByText(/All projects and databases deleted/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('create_backup');
  });
});
