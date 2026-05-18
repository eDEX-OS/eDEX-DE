import { useState, useEffect, useCallback } from 'preact/hooks';
import { listUnits, unitAction, getUnitLogs, getUnitStatus } from '../../ipc';
import type { SystemdUnit } from '../../ipc';

type UnitScope = 'system' | 'user';
type UnitTypeFilter = 'service' | 'socket' | 'timer' | 'all';

const SYSTEMD_UNAVAILABLE_MESSAGE = 'systemd tools not available — install systemd to enable service management.';

function formatServiceError(error: unknown) {
  const message = String(error);

  if (message.includes('No such file or directory')) {
    return SYSTEMD_UNAVAILABLE_MESSAGE;
  }

  if (message.includes('System has not been booted with systemd')) {
    return 'systemd not active in this session — boot with systemd as PID 1 to manage services.';
  }

  if (message.includes('journalctl not found')) {
    return 'journalctl not available — install systemd to view service logs.';
  }

  return `Service manager unavailable — ${message}`;
}

export function ServiceManager() {
  const [units, setUnits] = useState<SystemdUnit[]>([]);
  const [loading, setLoading] = useState(false);
  const [scope, setScope] = useState<UnitScope>('system');
  const [typeFilter, setTypeFilter] = useState<UnitTypeFilter>('service');
  const [search, setSearch] = useState('');
  const [selectedUnit, setSelectedUnit] = useState<string | null>(null);
  const [logs, setLogs] = useState('');
  const [logsLoading, setLogsLoading] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);
  const [availabilityMessage, setAvailabilityMessage] = useState<string>('');

  const load = useCallback(() => {
    setLoading(true);
    listUnits(scope === 'user', typeFilter === 'all' ? undefined : typeFilter)
      .then((nextUnits) => {
        setUnits(nextUnits);
        setAvailabilityMessage('');
      })
      .catch((error) => {
        console.error(error);
        setUnits([]);
        setSelectedUnit(null);
        setLogs('');
        setAvailabilityMessage(formatServiceError(error));
      })
      .finally(() => setLoading(false));
  }, [scope, typeFilter]);

  useEffect(() => {
    load();
  }, [load]);

  const loadLogs = useCallback(
    (unit: string) => {
      setSelectedUnit(unit);
      setLogsLoading(true);
      Promise.all([getUnitLogs(unit, 80), getUnitStatus(unit, scope === 'user')])
        .then(([unitLogs, unitStatus]) => {
          setLogs(`${unitStatus}\n\n${unitLogs}`.trim());
          setAvailabilityMessage('');
        })
        .catch((error) => {
          console.error(error);
          setLogs(formatServiceError(error));
        })
        .finally(() => setLogsLoading(false));
    },
    [scope],
  );

  const doAction = async (unit: string, action: string) => {
    try {
      const msg = await unitAction(unit, action, scope === 'user');
      setActionMsg(msg);
      setAvailabilityMessage('');
      window.setTimeout(() => setActionMsg(null), 3000);
      load();
      if (selectedUnit === unit) {
        loadLogs(unit);
      }
    } catch (error) {
      console.error(error);
      setActionMsg(formatServiceError(error));
      window.setTimeout(() => setActionMsg(null), 4000);
    }
  };

  const filtered = units.filter(
    (unit) =>
      !search ||
      unit.name.toLowerCase().includes(search.toLowerCase()) ||
      unit.description.toLowerCase().includes(search.toLowerCase()),
  );

  const stateColor = (state: string) => {
    switch (state) {
      case 'active':
        return 'var(--color-success)';
      case 'failed':
        return 'var(--color-error)';
      case 'activating':
        return 'var(--color-warning)';
      default:
        return 'var(--color-fg-muted)';
    }
  };

  return (
    <div class="service-manager">
      <div class="svc-toolbar">
        <div class="svc-scope-tabs">
          {(['system', 'user'] as UnitScope[]).map((value) => (
            <button key={value} class={`svc-tab ${scope === value ? 'active' : ''}`} onClick={() => setScope(value)}>
              {value.toUpperCase()}
            </button>
          ))}
        </div>
        <select
          class="svc-type-select"
          value={typeFilter}
          onChange={(event) => setTypeFilter((event.target as HTMLSelectElement).value as UnitTypeFilter)}
        >
          <option value="service">Services</option>
          <option value="socket">Sockets</option>
          <option value="timer">Timers</option>
          <option value="all">All units</option>
        </select>
        <input
          class="svc-search"
          type="text"
          placeholder="Filter..."
          value={search}
          onInput={(event) => setSearch((event.target as HTMLInputElement).value)}
        />
        <button class="svc-refresh" onClick={load} title="Refresh">
          ↺
        </button>
      </div>

      {actionMsg && <div class="svc-action-msg">{actionMsg}</div>}

      {availabilityMessage && !loading ? (
        <div class="sysinfo-unavailable svc-unavailable">
          <div class="sysinfo-unavailable-title">SERVICE BACKEND OFFLINE</div>
          <div class="sysinfo-unavailable-message">{availabilityMessage}</div>
          <div class="sysinfo-unavailable-hint">The SVC tab expects systemctl, journalctl, and pkexec for privileged system service control.</div>
        </div>
      ) : (
        <div class="svc-list">
          {loading && <div class="svc-loading">Loading units...</div>}
          {!loading && filtered.length === 0 && <div class="svc-empty">No units found</div>}
          {filtered.map((unit) => (
            <div
              key={unit.name}
              class={`svc-row ${selectedUnit === unit.name ? 'selected' : ''}`}
              onClick={() => loadLogs(unit.name)}
            >
              <div
                class="svc-status-dot"
                style={{ background: stateColor(unit.activeState) }}
                title={unit.activeState}
              />
              <div class="svc-info">
                <div class="svc-name">{unit.name}</div>
                <div class="svc-desc">{unit.description}</div>
              </div>
              <div class="svc-states">
                <span class="svc-state" style={{ color: stateColor(unit.activeState) }}>
                  {unit.activeState}
                </span>
                <span class="svc-substate">{unit.subState}</span>
              </div>
              <div class="svc-actions" onClick={(event) => event.stopPropagation()}>
                {unit.activeState !== 'active' && (
                  <button class="svc-btn start" onClick={() => doAction(unit.name, 'start')} title="Start">
                    ▶
                  </button>
                )}
                {unit.activeState === 'active' && (
                  <button class="svc-btn stop" onClick={() => doAction(unit.name, 'stop')} title="Stop">
                    ■
                  </button>
                )}
                <button class="svc-btn restart" onClick={() => doAction(unit.name, 'restart')} title="Restart">
                  ↺
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {selectedUnit && !availabilityMessage && (
        <div class="svc-logs">
          <div class="svc-logs-header">
            <span class="svc-logs-title">LOGS — {selectedUnit}</span>
            <button class="svc-logs-close" onClick={() => setSelectedUnit(null)}>
              ×
            </button>
          </div>
          <pre class="svc-logs-content">{logsLoading ? 'Loading...' : logs || 'No logs available'}</pre>
        </div>
      )}
    </div>
  );
}
