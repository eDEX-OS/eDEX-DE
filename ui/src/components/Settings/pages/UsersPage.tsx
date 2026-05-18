import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface UserInfo {
  username: string;
  uid: number;
  gid: number;
  home: string;
  shell: string;
  groups: string[];
}

export function UsersPage() {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [selected, setSelected] = useState('');
  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    invoke<UserInfo[]>('list_users').then((u) => {
      setUsers(u);
      if (u.length > 0) setSelected(u[0].username);
    }).catch(() => {});
  }, []);

  const changePassword = async () => {
    if (newPw !== confirmPw) {
      setStatus('Passwords do not match');
      return;
    }
    if (newPw.length < 6) {
      setStatus('Password must be at least 6 characters');
      return;
    }
    setBusy(true);
    try {
      await invoke('change_password', {
        username: selected,
        oldPassword: oldPw,
        newPassword: newPw,
      });
      setStatus('Password changed successfully');
      setOldPw('');
      setNewPw('');
      setConfirmPw('');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    } finally {
      setBusy(false);
    }
    setTimeout(() => setStatus(''), 5000);
  };

  const selectedUser = users.find((u) => u.username === selected);

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Users</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">User Accounts</h3>
        <div class="settings-row">
          <label class="settings-label">User</label>
          <select
            class="settings-select"
            value={selected}
            onChange={(e) => setSelected((e.target as HTMLSelectElement).value)}
          >
            {users.map((u) => (
              <option key={u.username} value={u.username}>{u.username}</option>
            ))}
          </select>
        </div>
        {selectedUser && (
          <>
            <div class="settings-info-block">
              <span class="settings-info-key">UID</span>
              <span class="settings-info-val">{selectedUser.uid}</span>
            </div>
            <div class="settings-info-block">
              <span class="settings-info-key">Home</span>
              <span class="settings-info-val">{selectedUser.home}</span>
            </div>
            <div class="settings-info-block">
              <span class="settings-info-key">Shell</span>
              <span class="settings-info-val">{selectedUser.shell}</span>
            </div>
            <div class="settings-info-block">
              <span class="settings-info-key">Groups</span>
              <span class="settings-info-val">{selectedUser.groups.join(', ')}</span>
            </div>
          </>
        )}
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Change Password</h3>
        <div class="settings-row">
          <label class="settings-label">Current Password</label>
          <input
            type="password"
            class="settings-input"
            value={oldPw}
            onInput={(e) => setOldPw((e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">New Password</label>
          <input
            type="password"
            class="settings-input"
            value={newPw}
            onInput={(e) => setNewPw((e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Confirm Password</label>
          <input
            type="password"
            class="settings-input"
            value={confirmPw}
            onInput={(e) => setConfirmPw((e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-actions">
          <button class="btn btn-primary" onClick={changePassword} disabled={busy || !selected}>
            {busy ? 'Changing...' : 'Change Password'}
          </button>
        </div>
      </div>

      {status && <p class="settings-status">{status}</p>}
    </div>
  );
}
