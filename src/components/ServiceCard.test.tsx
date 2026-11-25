/**
 * ServiceCard Component Tests
 * Phase 3: Process Manager UI
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { ServiceCard, reservedPortSuggestion } from './ServiceCard';
import { ServiceType, ServiceState, getDatabaseDisplayName } from '../types/services';
import { buildEnvBlock } from '../lib/envBlock';

describe('ServiceCard Component', () => {
  const mockHandlers = {
    onStart: vi.fn(),
    onStop: vi.fn(),
    onRestart: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('TC-PM-UI-01: Rendering', () => {
    it('should display service name', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );
      expect(screen.getByText('Caddy')).toBeInTheDocument();
    });

    it('should display service description', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );
      expect(screen.getByText('Web Server')).toBeInTheDocument();
    });

    it('should display service port', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );
      expect(screen.getByText(':8080')).toBeInTheDocument();
    });

    it('should have proper data-testid attributes for testing', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );
      expect(screen.getByTestId('service-card-caddy')).toBeInTheDocument();
      expect(screen.getByTestId('service-state-caddy')).toBeInTheDocument();
      expect(screen.getByTestId('start-button-caddy')).toBeInTheDocument();
    });
  });

  describe('TC-PM-UI-02: Stopped State', () => {
    it('should show gray badge when stopped', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );

      const badge = screen.getByTestId('service-state-caddy');
      expect(badge).toHaveTextContent('stopped');
      expect(badge).toHaveClass('status-gray');
    });

    it('should enable Start button when stopped', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );

      const startButton = screen.getByTestId('start-button-caddy');
      expect(startButton).not.toBeDisabled();
    });

    it('should disable Stop and Restart buttons when stopped', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );

      expect(screen.queryByTestId('stop-button-caddy')).not.toBeInTheDocument();
      expect(screen.queryByTestId('restart-button-caddy')).not.toBeInTheDocument();
    });

    it('should call onStart when Start button is clicked', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );

      const startButton = screen.getByTestId('start-button-caddy');
      fireEvent.click(startButton);
      expect(mockHandlers.onStart).toHaveBeenCalledTimes(1);
    });
  });

  describe('TC-PM-UI-03: Running State', () => {
    it('should show green badge when running', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Running}
          port={8080}
          {...mockHandlers}
        />
      );

      const badge = screen.getByTestId('service-state-caddy');
      expect(badge).toHaveTextContent('running');
      expect(badge).toHaveClass('status-green');
    });

    it('should disable Start button when running', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Running}
          port={8080}
          {...mockHandlers}
        />
      );

      expect(screen.queryByTestId('start-button-caddy')).not.toBeInTheDocument();
    });

    it('should enable Stop and Restart buttons when running', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Running}
          port={8080}
          {...mockHandlers}
        />
      );

      const stopButton = screen.getByTestId('stop-button-caddy');
      const restartButton = screen.getByTestId('restart-button-caddy');
      expect(stopButton).not.toBeDisabled();
      expect(restartButton).not.toBeDisabled();
    });

    it('should call onStop when Stop button is clicked', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Running}
          port={8080}
          {...mockHandlers}
        />
      );

      const stopButton = screen.getByTestId('stop-button-caddy');
      fireEvent.click(stopButton);
      expect(mockHandlers.onStop).toHaveBeenCalledTimes(1);
    });

    it('should call onRestart when Restart button is clicked', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Running}
          port={8080}
          {...mockHandlers}
        />
      );

      const restartButton = screen.getByTestId('restart-button-caddy');
      fireEvent.click(restartButton);
      expect(mockHandlers.onRestart).toHaveBeenCalledTimes(1);
    });
  });

  describe('TC-PM-UI-04: Starting State', () => {
    it('should show blue badge when starting', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Starting}
          port={8080}
          {...mockHandlers}
        />
      );

      const badge = screen.getByTestId('service-state-caddy');
      expect(badge).toHaveTextContent('starting');
      expect(badge).toHaveClass('status-blue');
    });

    it('should disable Start button when starting', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Starting}
          port={8080}
          {...mockHandlers}
        />
      );

      const startButton = screen.getByTestId('start-button-caddy');
      expect(startButton).toBeDisabled();
    });
  });

  describe('TC-PM-UI-05: Stopping State', () => {
    it('should show orange badge when stopping', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopping}
          port={8080}
          {...mockHandlers}
        />
      );

      const badge = screen.getByTestId('service-state-caddy');
      expect(badge).toHaveTextContent('stopping');
      expect(badge).toHaveClass('status-orange');
    });

    it('should disable Start button when stopping', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopping}
          port={8080}
          {...mockHandlers}
        />
      );

      // When stopping, isRunning is false, so Start button is shown but disabled
      const startButton = screen.getByTestId('start-button-caddy');
      expect(startButton).toBeDisabled();
    });
  });

  describe('TC-PM-UI-06: Error State', () => {
    it('should show red badge when in error state', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Error}
          port={8080}
          error="Port 8080 already in use"
          {...mockHandlers}
        />
      );

      const badge = screen.getByTestId('service-state-caddy');
      expect(badge).toHaveTextContent('error');
      expect(badge).toHaveClass('status-red');
    });

    it('should display error message', () => {
      const errorMessage = 'Port 8080 already in use';
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Error}
          port={8080}
          error={errorMessage}
          {...mockHandlers}
        />
      );

      expect(screen.getByText(errorMessage)).toBeInTheDocument();
    });

    it('should show Start button for retry when in error state', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Error}
          port={8080}
          error="Failed to start"
          {...mockHandlers}
        />
      );

      const startButton = screen.getByTestId('start-button-caddy');
      expect(startButton).not.toBeDisabled();
    });
  });

  describe('TC-PM-UI-07: All Service Types', () => {
    it('should render Caddy service card correctly', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.Caddy}
          state={ServiceState.Stopped}
          port={8080}
          {...mockHandlers}
        />
      );

      expect(screen.getByText('Caddy')).toBeInTheDocument();
      expect(screen.getByText(':8080')).toBeInTheDocument();
      expect(screen.getByText('Web Server')).toBeInTheDocument();
    });

    it('should render PHP-FPM service card correctly', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.PhpFpm}
          state={ServiceState.Stopped}
          port={9000}
          {...mockHandlers}
        />
      );

      expect(screen.getByText('PHP-FPM')).toBeInTheDocument();
      expect(screen.getByText(':9000')).toBeInTheDocument();
      expect(screen.getByText('PHP Runtime')).toBeInTheDocument();
    });

    it('should render MySQL service card correctly', () => {
      render(
        <ServiceCard
          serviceType={ServiceType.MySQL}
          state={ServiceState.Stopped}
          port={3307}
          {...mockHandlers}
        />
      );

      // MariaDB on Linux, MySQL elsewhere
      expect(screen.getByText(getDatabaseDisplayName())).toBeInTheDocument();
      expect(screen.getByText(':3307')).toBeInTheDocument();
      expect(screen.getByText('Database Server')).toBeInTheDocument();
    });
  });

  describe('Port reserved by Windows (spec 006 US3)', () => {
    // Exact text from process/manager.rs reserved_port_message.
    const reservedError =
      'Port 8000 (web server) is reserved by Windows (range 7949–8048), not used by a program. ' +
      'Switch to port 8049, or see "Reserved ports" in the manual for the permanent fix.';

    it('reads the suggested port only from a reserved-port error', () => {
      expect(reservedPortSuggestion(reservedError)).toBe(8049);
      expect(reservedPortSuggestion('Port 8080 is already in use by another program')).toBeNull();
      expect(reservedPortSuggestion(undefined)).toBeNull();
    });

    it('shows the whole message and a Use port button that reports the port', () => {
      const onUsePort = vi.fn();
      render(
        <ServiceCard serviceType={ServiceType.Caddy} state={ServiceState.Error} port={8000}
          error={reservedError} onUsePort={onUsePort} {...mockHandlers} />
      );
      expect(screen.getByText(reservedError)).toBeInTheDocument(); // not cut at 100 chars
      fireEvent.click(screen.getByTestId('use-port-button-caddy'));
      expect(onUsePort).toHaveBeenCalledWith(8049);
    });

    it('has no Use port button for an ordinary error', () => {
      render(
        <ServiceCard serviceType={ServiceType.Caddy} state={ServiceState.Error} port={8080}
          error="Port 8080 is already in use by another program" onUsePort={vi.fn()} {...mockHandlers} />
      );
      expect(screen.queryByTestId('use-port-button-caddy')).not.toBeInTheDocument();
    });
  });

  describe('Copy .env settings (spec 006 US4)', () => {
    const settings = { mysql_port: 13307, mysql_root_password: 's3cr$t"pw' };
    const expected = buildEnvBlock({ port: 13307, password: 's3cr$t"pw' });

    beforeEach(() => {
      vi.mocked(invoke).mockResolvedValue(settings);
    });

    it('is shown only on the database card', () => {
      render(<ServiceCard serviceType={ServiceType.Caddy} state={ServiceState.Stopped} {...mockHandlers} />);
      expect(screen.queryByTestId('copy-env-button')).not.toBeInTheDocument();
      render(<ServiceCard serviceType={ServiceType.MySQL} state={ServiceState.Stopped} {...mockHandlers} />);
      expect(screen.getByTestId('copy-env-button')).toBeInTheDocument();
    });

    it('is hidden when the database is not installed', () => {
      render(<ServiceCard serviceType={ServiceType.MySQL} state={ServiceState.Stopped} notInstalled {...mockHandlers} />);
      expect(screen.queryByTestId('copy-env-button')).not.toBeInTheDocument();
    });

    it('copies the current settings and never shows the password', async () => {
      const writeText = vi.fn().mockResolvedValue(undefined);
      Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
      render(<ServiceCard serviceType={ServiceType.MySQL} state={ServiceState.Running} {...mockHandlers} />);

      fireEvent.click(screen.getByTestId('copy-env-button'));

      await waitFor(() => expect(writeText).toHaveBeenCalledWith(expected));
      expect(invoke).toHaveBeenCalledWith('get_settings');
      expect(await screen.findByRole('status')).toHaveTextContent('Copied');
      expect(document.body.textContent).not.toContain('s3cr');
    });

    it('falls back to a masked box when the clipboard fails, revealed only on Show', async () => {
      const writeText = vi.fn().mockRejectedValue(new Error('denied'));
      Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
      render(<ServiceCard serviceType={ServiceType.MySQL} state={ServiceState.Running} {...mockHandlers} />);

      fireEvent.click(screen.getByTestId('copy-env-button'));

      const box = (await screen.findByTestId('env-block')) as HTMLTextAreaElement;
      expect(box.value).toContain('DB_PORT=13307');
      expect(box.value).toContain('DB_PASSWORD=••••');
      expect(box.value).not.toContain('s3cr');

      fireEvent.click(screen.getByText('Show password'));
      expect(box.value).toBe(expected);

      fireEvent.click(screen.getByText('Close'));
      expect(screen.queryByTestId('env-block')).not.toBeInTheDocument();
      expect(document.body.textContent).not.toContain('s3cr');
    });
  });
});
