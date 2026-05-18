import { ServiceManager } from '../../SystemServices';

export function SystemServicesPage() {
  return (
    <div class="settings-page">
      <h2 class="settings-page-title">System Services</h2>
      <ServiceManager />
    </div>
  );
}
