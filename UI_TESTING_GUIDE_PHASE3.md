# CAMPP Phase 3: UI Testing Guide

## Overview

This guide provides step-by-step instructions for manually testing the Phase 3 Process Manager UI functionality.

**Test Environment**: CAMPP Desktop Application running in development mode

**Prerequisites**:
1. Runtime binaries must be downloaded (complete First Run Wizard if needed)
2. Ports 8080, 9000, and 3307 must be available
3. Application launched with `npm run tauri dev`

---

## Quick Reference: Service States

| State | Color | Description |
|-------|-------|-------------|
| Stopped | Gray | Service is not running |
| Starting | Blue | Service is starting up |
| Running | Green | Service is actively running |
| Stopping | Orange | Service is shutting down |
| Error | Red | Service encountered an error |

---

## Test Case 1: Dashboard Initial Display

### Objective
Verify the dashboard displays correctly on application launch.

### Steps
1. Launch the application
2. Observe the dashboard

### Expected Results
- [ ] Application window opens successfully
- [ ] Dashboard displays three service cards in a grid layout
- [ ] Service cards are labeled: "Caddy", "PHP-FPM", "MariaDB"
- [ ] Each card shows a port number (8080, 9000, 3307)
- [ ] All services show "Stopped" state (gray badge)
- [ ] Each card has a "Start" button enabled
- [ ] Stop and Restart buttons are disabled
- [ ] No error messages displayed

### Actual Results
*Record your observations here*

---

## Test Case 2: Start Caddy Service

### Objective
Verify Caddy web server starts successfully.

### Steps
1. Locate the Caddy service card
2. Click the "Start" button
3. Observe the state transition

### Expected Results
- [ ] State changes to "Starting" (blue badge)
- [ ] Buttons become disabled during transition
- [ ] Within 2-5 seconds, state changes to "Running" (green badge)
- [ ] "Stop" and "Restart" buttons become enabled
- [ ] "Start" button becomes disabled
- [ ] No error message appears

### Verification Commands
```bash
# Check if Caddy is listening on port 8080
curl http://localhost:8080
# Expected: HTTP response (may be 404 or empty page)
```

### Actual Results
*Record your observations here*

---

## Test Case 3: Start PHP-FPM Service

### Objective
Verify PHP-FPM starts successfully.

### Steps
1. Locate the PHP-FPM service card
2. Click the "Start" button
3. Observe the state transition

### Expected Results
- [ ] State changes to "Starting" (blue badge)
- [ ] Within 2-5 seconds, state changes to "Running" (green badge)
- [ ] "Stop" and "Restart" buttons become enabled
- [ ] No error message appears

### Verification Commands
```bash
# Check if PHP-FPM is listening
netstat -an | findstr 9000  # Windows
lsof -i :9000               # macOS/Linux
# Expected: Port 9000 is in LISTEN state
```

### Actual Results
*Record your observations here*

---

## Test Case 4: Start MariaDB Service

### Objective
Verify MariaDB starts successfully.

### Steps
1. Locate the MariaDB service card
2. Click the "Start" button
3. Observe the state transition

### Expected Results
- [ ] State changes to "Starting" (blue badge)
- [ ] Within 10-20 seconds, state changes to "Running" (green badge)
- [ ] "Stop" and "Restart" buttons become enabled
- [ ] No error message appears
- [ ] Note: First run may take longer (15-30 seconds) for initialization

### Verification Commands
```bash
# Check if MariaDB is listening
netstat -an | findstr 3307  # Windows
lsof -i :3307               # macOS/Linux
# Expected: Port 3307 is in LISTEN state
```

### Actual Results
*Record your observations here*

---

## Test Case 5: Stop Running Service

### Objective
Verify a running service can be stopped.

### Steps
1. Start any service (if not already running)
2. Verify service shows "Running" state
3. Click the "Stop" button
4. Observe the state transition

### Expected Results
- [ ] State changes to "Stopping" (orange badge)
- [ ] Buttons become disabled during transition
- [ ] Within 2-5 seconds, state changes to "Stopped" (gray badge)
- [ ] "Start" button becomes enabled
- [ ] "Stop" and "Restart" buttons become disabled

### Actual Results
*Record your observations here*

---

## Test Case 6: Restart Running Service

### Objective
Verify a running service can be restarted.

### Steps
1. Ensure a service is running
2. Click the "Restart" button
3. Observe the state transitions

### Expected Results
- [ ] State changes to "Stopping" (orange badge)
- [ ] Briefly shows "Stopped" state
- [ ] Changes to "Starting" (blue badge)
- [ ] Returns to "Running" (green badge)
- [ ] Total transition time: 5-10 seconds

### Actual Results
*Record your observations here*

---

## Test Case 7: Start All Services

### Objective
Verify all services can run simultaneously.

### Steps
1. Ensure all services are stopped
2. Start Caddy
3. Start PHP-FPM
4. Start MariaDB
5. Verify all services show "Running" state

### Expected Results
- [ ] All three services show "Running" state (green badges)
- [ ] Each service has "Stop" and "Restart" buttons enabled
- [ ] No port conflict errors
- [ ] UI remains responsive

### Verification
Open browser to `http://localhost:8080` - should receive a response from Caddy.

### Actual Results
*Record your observations here*

---

## Test Case 8: Auto-Refresh Status

### Objective
Verify service status updates automatically.

### Steps
1. Start a service
2. Note the current time
3. Wait 2-5 seconds
4. Check if status updates

### Expected Results
- [ ] Status refreshes automatically every 2 seconds
- [ ] UI shows latest service state
- [ ] No manual refresh required

### Advanced Test
1. Kill a service process externally
2. Wait 2-4 seconds
3. Verify UI updates to show "Stopped" or "Error" state

### Actual Results
*Record your observations here*

---

## Test Case 9: Error Handling - Missing Runtime

### Objective
Verify error handling when runtime binaries are missing.

### Steps
1. Stop all services
2. Access Debug menu (if available)
3. Select "Reset Installation"
4. Try to start a service

### Expected Results
- [ ] Service transitions to "Error" state (red badge)
- [ ] Error message is displayed on the service card
- [ ] Error describes the issue (e.g., "Runtime binary not found")
- [ ] "Start" button remains enabled to retry

### Recovery
Run the download wizard again to restore runtime binaries.

### Actual Results
*Record your observations here*

---

## Test Case 10: Error Handling - Port Conflict

### Objective
Verify error handling when port is already in use.

### Steps
1. Start a service (e.g., Caddy on port 8080)
2. Use external tool to bind port 8080
3. Observe the error state

### Expected Results
- [ ] Service transitions to "Error" state (red badge)
- [ ] Error message indicates port conflict
- [ ] Service can be recovered by stopping conflicting service

### Actual Results
*Record your observations here*

---

## Test Case 11: Concurrent Operations

### Objective
Verify UI handles multiple simultaneous operations.

### Steps
1. Start all three services in quick succession
2. While services are starting, try stopping one
3. Try restarting another

### Expected Results
- [ ] UI remains responsive
- [ ] Buttons disable appropriately during transitions
- [ ] No crashes or freezes
- [ ] Each service maintains correct state

### Actual Results
*Record your observations here*

---

## Integration Test: Full Stack

### Objective
Verify end-to-end functionality with all services running.

### Steps
1. Start all three services
2. Open browser to `http://localhost:8080`
3. Verify Caddy responds
4. Check logs for errors

### Expected Results
- [ ] Browser receives response from Caddy
- [ ] All services show "Running" state
- [ ] No errors in service logs
- [ ] Ports 8080, 9000, 3307 are in LISTEN state

### Actual Results
*Record your observations here*

---

## Verification Commands Reference

### Check Service Status

```bash
# Windows - Check all service ports
netstat -an | findstr "8080 9000 3307"

# macOS/Linux - Check all service ports
lsof -i :8080 -i :9000 -i :3307

# Check running processes
# Windows
tasklist | findstr /I "caddy php mysqld"

# macOS/Linux
ps aux | grep -E "caddy|php|mysqld"
```

### View Logs

```bash
# Windows
type C:\Users\<user>\.campp\logs\caddy.log
type C:\Users\<user>\.campp\logs\php-fpm.log
type C:\Users\<user>\.campp\logs\mariadb.log

# macOS/Linux
tail -f ~/.campp/logs/caddy.log
tail -f ~/.campp/logs/php-fpm.log
tail -f ~/.campp/logs/mariadb.log
```

### Test Web Server

```bash
# Test Caddy is responding
curl http://localhost:8080

# Test with verbose output
curl -v http://localhost:8080
```

---

## Known Issues and Limitations

1. **MariaDB First Run**: First MariaDB start may take 15-30 seconds for data directory initialization
2. **PHP Implementation**: Currently using PHP-CGI instead of full PHP-FPM for MVP
3. **No State Persistence**: Services don't persist across app restarts (Phase 4)
4. **No Auto-Start**: Services don't auto-start on application launch (Phase 4)

---

## Test Results Summary

| Test Case | Status | Notes |
|-----------|--------|-------|
| TC-PM-01: Dashboard Initial Display | PASS/FAIL | |
| TC-PM-02: Start Caddy Service | PASS/FAIL | |
| TC-PM-03: Start PHP-FPM Service | PASS/FAIL | |
| TC-PM-04: Start MariaDB Service | PASS/FAIL | |
| TC-PM-05: Stop Running Service | PASS/FAIL | |
| TC-PM-06: Restart Running Service | PASS/FAIL | |
| TC-PM-07: Start All Services | PASS/FAIL | |
| TC-PM-08: Auto-Refresh Status | PASS/FAIL | |
| TC-PM-09: Error - Missing Runtime | PASS/FAIL | |
| TC-PM-10: Error - Port Conflict | PASS/FAIL | |
| TC-PM-11: Concurrent Operations | PASS/FAIL | |

**Overall Result**: _____ PASS/FAIL

---

## Issues Found

1. *Document any issues discovered during testing*

2.

3.

---

## Recommendations

1. *Record any recommendations for improvements*

2.

3.

---

## Next Steps

After completing Phase 3 testing:

1. Document all bugs in GitHub Issues
2. Fix critical issues before proceeding to Phase 4
3. Implement improvements based on test findings
4. Proceed to Phase 4: Configuration Generation

---

*Test Execution Date*: _______________
*Tester*: _______________
*Application Version*: _______________
