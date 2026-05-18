import { useEffect, useState } from 'preact/hooks';
import { useSettings } from '../context';
import { padStart } from '../utils';

export function TopBar() {
  const { settings } = useSettings();
  const [time, setTime] = useState('');
  const [date, setDate] = useState('');

  useEffect(() => {
    const tick = () => {
      const now = new Date();
      const h = settings.clockHours === 12 ? (now.getHours() % 12 || 12) : now.getHours();
      const suffix = settings.clockHours === 12 ? (now.getHours() >= 12 ? ' PM' : ' AM') : '';

      setTime(`${padStart(h)}:${padStart(now.getMinutes())}:${padStart(now.getSeconds())}${suffix}`);
      setDate(
        now.toLocaleDateString(undefined, {
          weekday: 'short',
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        }),
      );
    };

    tick();
    const id = window.setInterval(tick, 1000);
    return () => window.clearInterval(id);
  }, [settings.clockHours]);

  return (
    <div class="topbar">
      <div class="topbar-left">
        <span class="topbar-brand">eDEX-DE</span>
      </div>
      <div class="topbar-center">
        <span class="topbar-time">{time}</span>
      </div>
      <div class="topbar-right">
        <span class="topbar-date">{date}</span>
      </div>
    </div>
  );
}
