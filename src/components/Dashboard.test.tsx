/**
 * Dashboard Component Tests
 * Phase 3: Process Manager UI
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { Dashboard } from './Dashboard';
import { ServiceType, ServiceState } from '../types/services';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Dashboard Component', () => {
  const mockServiceMap = {
    [ServiceType.Caddy]: {
      service_type: ServiceType.Caddy,
      state: ServiceState.Stopped,
      port: 8080,
      error_message: null,
    },
    [ServiceType.PhpFpm]: {
      service_type: ServiceType.PhpFpm,
      state: ServiceState.Stopped,
      port: 9000,
      error_message: null,
    },
    [ServiceType.MySQL]: {
      service_type: ServiceType.MySQL,
      state: ServiceState.Stopped,
      port: 3307,
      error_message: null,
    },
  };

  // Answer each backend command with a realistic value. Returning the service map
  // for every command made get_install_dir hand an object to a <span> and crashed
  // the render.
  const installed = { caddy: '2.11.7', php: '8.5.12', mysql: '8.4.11' };
  const mockBackend = (statuses: unknown, overrides: Record<string, unknown> = {}) => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd in overrides) {
        const v = overrides[cmd];
        if (v instanceof Error) throw v;
        return v;
      }
      switch (cmd) {
        case 'get_all_statuses': return statuses;
        case 'get_installed_versions': return installed;
        case 'get_settings': return { listen_all_interfaces: false, theme: 'system', project_root: 'C:/CAMPP/projects' };
        case 'get_install_dir': return 'C:/CAMPP';
        default: return null;
      }
    });
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // Don't use fake timers - they cause issues with async/await in tests
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('TC-PM-DASH-01: Initial Display', () => {
    it('should render all three service cards', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      // Wait for initial fetch
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_all_statuses');
      });

      // Check service cards are rendered
      expect(await screen.findByTestId('service-card-caddy')).toBeInTheDocument();
      expect(screen.getByTestId('service-card-php-fpm')).toBeInTheDocument();
      expect(screen.getByTestId('service-card-mysql')).toBeInTheDocument();
    });

    it('should fetch service statuses on mount', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_all_statuses');
      });
    });

    it('should display status bar', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      await waitFor(() => {
        const statusBar = screen.getByTestId('status-bar');
        expect(statusBar).toBeInTheDocument();
      });
    });
  });

  describe('TC-PM-DASH-02: Status Refresh', () => {
    it('should call get_all_statuses multiple times', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      // Wait for initial fetch
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_all_statuses');
      });

      // Wait a bit and check that it was called again (due to interval)
      await waitFor(
        () => {
          expect(vi.mocked(invoke).mock.calls.length).toBeGreaterThan(1);
        },
        { timeout: 3000 }
      );
    });
  });

  describe('TC-PM-DASH-03: Start Service', () => {
    it('should call start_service when Start button is clicked', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      await waitFor(() => {
        const startButton = screen.getByTestId('start-button-caddy');
        fireEvent.click(startButton);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('start_service', { service: ServiceType.Caddy });
      });
    });
  });

  describe('TC-PM-DASH-04: Stop Service', () => {
    it('should call stop_service when Stop button is clicked', async () => {
      const runningMap = {
        ...mockServiceMap,
        [ServiceType.Caddy]: {
          ...mockServiceMap[ServiceType.Caddy],
          state: ServiceState.Running,
        },
      };

      mockBackend(runningMap);

      render(<Dashboard />);

      await waitFor(() => {
        const stopButton = screen.getByTestId('stop-button-caddy');
        fireEvent.click(stopButton);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('stop_service', { service: ServiceType.Caddy });
      });
    });
  });

  describe('TC-PM-DASH-05: Restart Service', () => {
    it('should call restart_service when Restart button is clicked', async () => {
      const runningMap = {
        ...mockServiceMap,
        [ServiceType.Caddy]: {
          ...mockServiceMap[ServiceType.Caddy],
          state: ServiceState.Running,
        },
      };

      mockBackend(runningMap);

      render(<Dashboard />);

      await waitFor(() => {
        const restartButton = screen.getByTestId('restart-button-caddy');
        fireEvent.click(restartButton);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('restart_service', { service: ServiceType.Caddy });
      });
    });
  });

  describe('TC-PM-DASH-06: Error Handling', () => {
    it('should display error when service fails to start', async () => {
      const errorMessage = 'Port 8080 already in use';
      mockBackend({
        ...mockServiceMap,
        [ServiceType.Caddy]: {
          ...mockServiceMap[ServiceType.Caddy],
          state: ServiceState.Error,
          error_message: errorMessage,
        },
      });

      render(<Dashboard />);

      await waitFor(() => {
        expect(screen.getByText(errorMessage)).toBeInTheDocument();
      });
    });

    it('should show error state badge when service fails', async () => {
      mockBackend({
        ...mockServiceMap,
        [ServiceType.PhpFpm]: {
          ...mockServiceMap[ServiceType.PhpFpm],
          state: ServiceState.Error,
          error_message: 'Failed to start',
        },
      });

      render(<Dashboard />);

      await waitFor(() => {
        const badge = screen.getByTestId('service-state-php-fpm');
        expect(badge).toHaveTextContent('error');
      });
    });
  });

  describe('TC-PM-DASH-07: Quick Actions', () => {
    it('should display version information', async () => {
      mockBackend(mockServiceMap);

      render(<Dashboard />);

      await waitFor(() => {
        expect(screen.getByText(/^CAMPP v\d+\.\d+\.\d+$/)).toBeInTheDocument();
      });
    });
  });

  describe('TC-PM-DASH-08: All Services Running', () => {
    it('should show all services as running when all started', async () => {
      const allRunningMap = {
        [ServiceType.Caddy]: {
          ...mockServiceMap[ServiceType.Caddy],
          state: ServiceState.Running,
        },
        [ServiceType.PhpFpm]: {
          ...mockServiceMap[ServiceType.PhpFpm],
          state: ServiceState.Running,
        },
        [ServiceType.MySQL]: {
          ...mockServiceMap[ServiceType.MySQL],
          state: ServiceState.Running,
        },
      };

      mockBackend(allRunningMap);

      render(<Dashboard />);

      await waitFor(() => {
        expect(screen.getByTestId('service-state-caddy')).toHaveTextContent('running');
        expect(screen.getByTestId('service-state-php-fpm')).toHaveTextContent('running');
        expect(screen.getByTestId('service-state-mysql')).toHaveTextContent('running');
      });
    });
  });
});
