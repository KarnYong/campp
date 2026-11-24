import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { SitesPage } from './SitesPage';
import type { SitesResponse } from '../types/services';

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(() => Promise.resolve()),
  revealItemInDir: vi.fn(() => Promise.resolve()),
}));

// Example from specs/005-local-sites-https/contracts/list-sites.md
const response: SitesResponse = {
  projects_dir: 'C:\\CAMPP\\projects',
  web_port: 8080,
  sites: [
    { name: 'blog', host: 'blog.localhost', url: 'http://blog.localhost:8080', kind: 'plain', status: 'ok' },
    { name: 'shop', host: 'shop.localhost', url: 'http://shop.localhost:8080', kind: 'framework', status: 'ok' },
    { name: 'My Notes', host: null, url: null, kind: 'plain', status: 'invalid_name' },
  ],
};

describe('SitesPage (spec 005 US2)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(response);
  });

  it('lists sites with addresses and opens one in the browser', async () => {
    render(<SitesPage webRunning />);
    expect(await screen.findByText('http://blog.localhost:8080')).toBeInTheDocument();
    expect(screen.getByText('http://shop.localhost:8080')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('list_sites');

    fireEvent.click(screen.getByRole('button', { name: 'Open blog' }));
    expect(openUrl).toHaveBeenCalledWith('http://blog.localhost:8080');
  });

  it('marks Laravel/Symfony folders', async () => {
    render(<SitesPage webRunning />);
    const shop = (await screen.findByText('http://shop.localhost:8080')).closest('.list-row')!;
    expect(shop).toHaveTextContent('Laravel/Symfony');
    const blog = screen.getByText('http://blog.localhost:8080').closest('.list-row')!;
    expect(blog).not.toHaveTextContent('Laravel/Symfony');
  });

  it('explains a folder that cannot have an address', async () => {
    render(<SitesPage webRunning />);
    const row = (await screen.findByText('My Notes')).closest('.list-row')!;
    expect(row).toHaveTextContent('Rename using letters, digits and hyphens');
    expect(row).toHaveTextContent('http://localhost:8080/My Notes/');
    expect(screen.queryByRole('button', { name: 'Open My Notes' })).not.toBeInTheDocument();
  });

  it('disables Open while the web server is stopped', async () => {
    render(<SitesPage webRunning={false} />);
    expect(await screen.findByRole('button', { name: 'Open blog' })).toBeDisabled();
    expect(screen.getByText('Start the web server to open sites.')).toBeInTheDocument();
  });

  it('shows an empty state with the projects folder button', async () => {
    vi.mocked(invoke).mockResolvedValue({ ...response, sites: [] });
    render(<SitesPage webRunning />);
    expect(await screen.findByText(/Put each project in its own folder/)).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole('button', { name: 'Open projects folder' })[0]);
    expect(revealItemInDir).toHaveBeenCalledWith('C:\\CAMPP\\projects');
  });

  it('reloads when the window gets focus', async () => {
    render(<SitesPage webRunning />);
    await screen.findByText('http://blog.localhost:8080');
    fireEvent.focus(window);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});
